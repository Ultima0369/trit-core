//! Application facade — shared between CLI and Tauri.
//!
//! Orchestrates the two pipeline links (analysis + attention)
//! and presentation rendering in one call. Both the CLI binary
//! and Tauri commands use this same entry point.
//!
//! M2: Added LLM perception support via `PerceptChain`.

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::bc::presentation::{AuroraRenderer, ConflictCard, ViewState};
use crate::bc::relationship_annotation::{ContactInput, ContactProfile};
use crate::config::ConfigStore;
use crate::db::Database;
use crate::ingest::json_fallback::JsonFallbackSource;
use crate::percept::cloud::CloudLLMProvider;
use crate::percept::fft::FFTProvider;
use crate::percept::local::LocalLLMProvider;
use crate::percept::PerceptChain;
use crate::percept::RetrospectiveDoc;
use crate::pipeline::analysis::{self, AnalysisReport, PhaseTrajectory, SignalSpec};
use crate::pipeline::attention::{self, AttentionOutcome};

/// Input parameters for a single pipeline run.
#[derive(Debug, Clone)]
pub struct AnalysisInput {
    pub spec: SignalSpec,
    pub frequency_threshold: f64,
    pub user_feels_normal: bool,
}

/// Complete output of a pipeline run.
#[derive(Debug, Clone)]
pub struct AppOutput {
    pub analysis_report: AnalysisReport,
    pub attention_outcome: AttentionOutcome,
    pub html: String,
    pub json: String,
}

/// Application facade — owns the database connection, loaded contacts,
/// configuration store, and the perception chain.
pub struct AuroraApp {
    db: Arc<Mutex<Database>>,
    contacts: Vec<ContactProfile>,
    percept_chain: PerceptChain,
    config: Arc<ConfigStore>,
    /// Accumulated phase trajectory across analysis runs (Lever 3 stagnation detection).
    trajectory: Mutex<Option<PhaseTrajectory>>,
}

impl AuroraApp {
    /// Create a new AuroraApp with a database connection and perception chain.
    ///
    /// If `db_path` is `None` or `":memory:"`, opens an in-memory database.
    /// Otherwise opens (or creates) the SQLite database at the given path.
    ///
    /// The perception chain is initialized with:
    /// - CloudLLMProvider (if API key configured)
    /// - LocalLLMProvider (if endpoint configured)
    /// - FFTProvider (always — never-offline safety floor)
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        let db = match db_path {
            None => Database::open_in_memory()?,
            Some(p) if p == Path::new(":memory:") => Database::open_in_memory()?,
            Some(p) => Database::open(p)?,
        };

        let config = Arc::new(ConfigStore::open()?);

        // Build perception chain with available providers
        let mut chain = PerceptChain::new();

        // Cloud LLM: only add if a cloud model is configured with an API key
        if let Ok(Some(cloud_model)) = config.cloud_model() {
            match CloudLLMProvider::new(config.clone(), &cloud_model) {
                Ok(provider) => {
                    chain = chain.with(Box::new(provider));
                }
                Err(e) => {
                    tracing::warn!("cloud LLM not available: {e}");
                }
            }
        }

        // Local LLM: only add if endpoint is configured
        if config.local_model_path().ok().flatten().is_some() {
            match LocalLLMProvider::new(config.clone()) {
                Ok(provider) => {
                    chain = chain.with(Box::new(provider));
                }
                Err(e) => {
                    tracing::warn!("local LLM not available: {e}");
                }
            }
        }

        // FFTProvider always — never-offline safety floor.
        // ponytail: fixed fallback spec; the FFT provider is the last-resort
        // signal source when no LLM is configured, so a deterministic default
        // is intentional. Promote to config if real signal input is needed.
        const FALLBACK_FREQ: f64 = 2.0;
        const FALLBACK_SAMPLE_RATE: f64 = 100.0;
        const FALLBACK_DURATION_SECS: f64 = 1.0;
        chain = chain.with(Box::new(FFTProvider::new(SignalSpec {
            freq: FALLBACK_FREQ,
            sample_rate: FALLBACK_SAMPLE_RATE,
            duration_secs: FALLBACK_DURATION_SECS,
            noise_std: 0.0,
        })));

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            contacts: Vec::new(),
            percept_chain: chain,
            config,
            trajectory: Mutex::new(None),
        })
    }

    /// Load contacts from a JSON data source file.
    ///
    /// Returns the number of contacts loaded.
    pub fn load_contacts(&mut self, data_source: &Path) -> Result<usize> {
        let source = JsonFallbackSource::new(data_source)?;
        let count = source.contact_count();
        let inputs: Vec<ContactInput> = source.load()?;
        self.contacts = inputs.into_iter().map(ContactProfile::from).collect();
        Ok(count)
    }

    /// Run the full analysis + attention pipeline WITHOUT LLM perception.
    ///
    /// This is the original pipeline path — FFT-only signal analysis.
    /// Takes `&self` so the app can be reused across multiple invocations
    /// (e.g., from Tauri command handlers).
    pub fn run_pipeline(&self, input: AnalysisInput) -> Result<AppOutput> {
        let contact_signals = analysis::contacts_to_tritwords(&self.contacts);

        let analysis_report = analysis::run_analysis(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &contact_signals,
        )
        .map_err(|e| anyhow::anyhow!("analysis link failed [{}/{}]: {e}", e.kind(), e))?;

        let db = self.db.lock().expect("db mutex poisoned");
        let attention_outcome = attention::run_attention(
            &analysis_report.decision,
            &analysis_report.decision.input_signals,
            &db,
            &self.contacts,
        )
        .map_err(|e| anyhow::anyhow!("attention link failed [{}/{}]: {e}", e.kind(), e))?;

        // Update phase trajectory for stagnation detection (Lever 3).
        if let Ok(mut traj) = self.trajectory.lock() {
            match traj.as_mut() {
                Some(t) => t.update(&analysis_report),
                None => *traj = Some(PhaseTrajectory::new(&analysis_report)),
            }
        }

        Self::render_output(analysis_report, attention_outcome)
    }

    /// Run the full pipeline WITH LLM perception.
    ///
    /// The perception chain tries cloud LLM -> local LLM -> FFT in order.
    /// Percept signals are merged into the analysis alongside embodied,
    /// individual, and contact signals.
    pub fn run_with_percept(&self, input: AnalysisInput, user_text: &str) -> Result<AppOutput> {
        let contact_signals = analysis::contacts_to_tritwords(&self.contacts);

        // Step 0: Perceive via degradation chain
        let percept = self
            .percept_chain
            .perceive_or_degrade(user_text)
            .map_err(|e| anyhow::anyhow!("perception failed [{}/{}]: {e}", e.kind(), e))?;

        // Step 1: Analysis with percept signals
        let analysis_report = analysis::run_analysis_from_percept(
            &input.spec,
            input.frequency_threshold,
            input.user_feels_normal,
            &contact_signals,
            &percept.signals,
        )
        .map_err(|e| anyhow::anyhow!("analysis link failed [{}/{}]: {e}", e.kind(), e))?;

        // Step 2: Attention
        let db = self.db.lock().expect("db mutex poisoned");
        let attention_outcome = attention::run_attention(
            &analysis_report.decision,
            &analysis_report.decision.input_signals,
            &db,
            &self.contacts,
        )
        .map_err(|e| anyhow::anyhow!("attention link failed [{}/{}]: {e}", e.kind(), e))?;

        // Update phase trajectory for stagnation detection (Lever 3).
        if let Ok(mut traj) = self.trajectory.lock() {
            match traj.as_mut() {
                Some(t) => t.update(&analysis_report),
                None => *traj = Some(PhaseTrajectory::new(&analysis_report)),
            }
        }

        Self::render_output(analysis_report, attention_outcome)
    }

    /// Get a reference to the config store (for CLI config commands).
    pub fn config(&self) -> &Arc<ConfigStore> {
        &self.config
    }

    /// Export all user data as a JSON string.
    ///
    /// M1 Exit Criteria "数据导出" + CHARTER "不剥夺" 底线：用户可带走全部
    /// 自己的数据离开系统。遍历 5 张表，每表导出为 `{columns, rows}`。
    /// ponytail: 通用 SELECT * + 列名反射，避免为每张表手写 struct。
    /// schema_version 表是系统内部元数据，不导出。
    pub fn export_data_json(&self) -> Result<String> {
        let db = self.db.lock().expect("db mutex poisoned");
        let conn = db.conn();
        let tables = [
            "contacts",
            "communication_events",
            "frame_annotations",
            "annotation_history",
            "audit_log",
        ];
        let mut export = serde_json::Map::new();
        for table in tables {
            export.insert(table.to_string(), dump_table(conn, table)?);
        }
        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
            export,
        ))?)
    }

    /// Get the current phase trajectory summary (Lever 3 stagnation detection).
    ///
    /// Returns `None` if no analysis runs have been recorded yet.
    /// When `is_stagnating` is true, the user's decision pattern hasn't
    /// meaningfully changed across recent runs — the mirror should reflect this.
    pub fn trajectory_summary(&self) -> Option<crate::pipeline::analysis::TrajectorySummary> {
        let traj = self.trajectory.lock().ok()?;
        traj.as_ref().map(|t| t.summary())
    }

    /// Whether the decision trajectory shows stagnation (Lever 3).
    pub fn is_stagnating(&self) -> bool {
        self.trajectory
            .lock()
            .ok()
            .and_then(|t| t.as_ref().map(|t| t.is_stagnating()))
            .unwrap_or(false)
    }

    /// Run a future retrospective simulation (Lever 2).
    ///
    /// Loads an SSP pathway scenario, formats a future-retrospective prompt
    /// from the user's decision, and runs it through the percept chain.
    /// Returns a [`RetrospectiveDoc`] with the 2066-perspective analysis.
    pub fn run_retrospective(
        &self,
        ssp_scenario_path: &Path,
        user_decision: &str,
    ) -> Result<RetrospectiveDoc> {
        use crate::percept::retrospective::SspScenario;
        let scenario = SspScenario::load(ssp_scenario_path)
            .map_err(|e| anyhow::anyhow!("failed to load SSP scenario [{}/{}]: {e}", e.kind(), e))?;
        let prompt = scenario.build_prompt(user_decision);
        let batch = self
            .percept_chain
            .perceive_or_degrade(&prompt)
            .map_err(|e| anyhow::anyhow!("retrospective perception failed [{}/{}]: {e}", e.kind(), e))?;
        // ponytail: build a minimal provider just to construct the doc.
        // The inner FFTProvider is irrelevant — we already have the batch from
        // percept_chain. RetrospectiveProvider exists for the trait impl;
        // for direct use, SspScenario::build_prompt + to_doc_from_batch is sufficient.
        Ok(RetrospectiveDoc {
            pathway: scenario.ssp_pathway.clone(),
            lookback_year: scenario.lookback_year,
            decision_prompt: scenario.decision_prompt.clone(),
            signal_count: batch.signals.len(),
            source: batch.source.clone(),
            confidence: batch.confidence,
            projected: scenario.projected_signals_2066.clone(),
        })
    }

    /// Render analysis + attention into HTML and JSON output.
    fn render_output(
        analysis_report: AnalysisReport,
        attention_outcome: AttentionOutcome,
    ) -> Result<AppOutput> {
        let mut view = ViewState::new(
            format!(
                "Detected frequency: {:.3} Hz | Decision: {:?}",
                analysis_report.spectrum.fundamental_hz,
                analysis_report.decision.result.value()
            ),
            attention_outcome.session.clone(),
        );

        for interrupt in &analysis_report.decision.interrupts {
            let (frame_a, frame_b) = interrupt.frames();
            view.add_conflict(ConflictCard {
                conflict_type: format!("{:?}", interrupt.conflict),
                reason: interrupt.reason.clone(),
                frame_a,
                frame_b,
                acknowledged: false,
            });
        }

        let renderer = AuroraRenderer;
        let html = renderer.render_html(&view);
        let json = renderer
            .render_json(&view)
            .map_err(|e| anyhow::anyhow!("JSON render failed: {e}"))?;

        Ok(AppOutput {
            analysis_report,
            attention_outcome,
            html,
            json,
        })
    }
}

/// Dump a table's rows as `{columns: [...], rows: [[...], ...]}` JSON.
///
/// ponytail: 反射列名 + 值类型，通用化——避免为每张表手写 struct。
/// 值用 rusqlite::types::Value 转 serde_json::Value（NULL→null, INTEGER→number,
/// TEXT→string, REAL→number, BLOB→hex string）。
fn dump_table(conn: &rusqlite::Connection, table: &str) -> Result<serde_json::Value> {
    use rusqlite::types::Value as V;
    // Defense-in-depth: whitelist of known table names. Even though callers
    // (export_data_json) use a hardcoded array, this guard prevents SQL
    // injection if a new caller passes user-controlled table names.
    const ALLOWED_TABLES: &[&str] = &[
        "contacts",
        "communication_events",
        "frame_annotations",
        "annotation_history",
        "audit_log",
    ];
    if !ALLOWED_TABLES.contains(&table) {
        anyhow::bail!("invalid table name: {table}");
    }
    let mut stmt = conn.prepare(&format!("SELECT * FROM {table}"))?;
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut row_iter = stmt.query([])?;
    while let Some(row) = row_iter.next()? {
        let mut values: Vec<serde_json::Value> = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            let v: V = row.get(i).unwrap_or(V::Null);
            values.push(sql_value_to_json(v));
        }
        rows.push(serde_json::Value::Array(values));
    }
    Ok(serde_json::json!({ "columns": columns, "rows": rows }))
}

/// rusqlite 值 → serde_json 值。
fn sql_value_to_json(v: rusqlite::types::Value) -> serde_json::Value {
    use rusqlite::types::Value as V;
    match v {
        V::Null => serde_json::Value::Null,
        V::Integer(i) => serde_json::json!(i),
        V::Real(r) => serde_json::json!(r),
        V::Text(s) => serde_json::Value::String(s),
        // BLOB → 十六进制字符串（用户表无 BLOB 列，此处仅 defense-in-depth）。
        V::Blob(b) => {
            serde_json::Value::String(b.iter().map(|byte| format!("{byte:02x}")).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_empty_db_has_all_tables() {
        let app = AuroraApp::new(None).unwrap();
        let json = app.export_data_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = parsed.as_object().unwrap();
        // 5 张表都在，即使空也有 columns 字段。
        for table in [
            "contacts",
            "communication_events",
            "frame_annotations",
            "annotation_history",
            "audit_log",
        ] {
            assert!(obj.contains_key(table), "missing table {table}");
            let t = obj.get(table).unwrap();
            assert!(t.get("columns").is_some(), "{table} missing columns");
            assert_eq!(
                t["rows"].as_array().unwrap().len(),
                0,
                "{table} should be empty"
            );
        }
    }

    #[test]
    fn export_includes_inserted_rows() {
        let app = AuroraApp::new(None).unwrap();
        {
            let db = app.db.lock().unwrap();
            db.conn()
                .execute(
                    "INSERT INTO contacts (id, name, relation_label, notes, deleted, created_at, updated_at)
                     VALUES ('c1', 'Alice', 'friend', '', 0, '2026-01-01', '2026-01-01')",
                    [],
                )
                .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO audit_log (timestamp, event_type, session_id, domain, description)
                     VALUES ('2026-01-01', 'TEST', 's1', NULL, 'test event')",
                    [],
                )
                .unwrap();
        }
        let json = app.export_data_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["contacts"]["rows"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["audit_log"]["rows"].as_array().unwrap().len(), 1);
        // 验证列名反射 + 值类型。
        let contact_cols = parsed["contacts"]["columns"].as_array().unwrap();
        assert!(contact_cols.iter().any(|c| c == "name"));
        let first_row = &parsed["contacts"]["rows"][0].as_array().unwrap();
        assert!(first_row.iter().any(|v| v == "Alice"));
    }
}
