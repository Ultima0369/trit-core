#!/usr/bin/env python3
"""Check for broken internal Markdown links in docs/."""
import re
from pathlib import Path

DOCS_ROOT = Path("docs")
ROOT = Path(".").resolve()


def collect_targets():
    """Collect files and directories that may legitimately be linked."""
    targets = set()

    # Everything under docs/
    for p in DOCS_ROOT.rglob("*"):
        rel = p.resolve().relative_to(ROOT).as_posix()
        targets.add(rel)
        if p.is_dir():
            targets.add(rel.rstrip("/") + "/")

    # Top-level markdown files reachable from docs (CHANGELOG.md, SECURITY.md, README.md, ...)
    for p in ROOT.iterdir():
        if p.is_file() and p.suffix == ".md":
            targets.add(p.name)

    # audit_log/ is linked from docs/INDEX.md
    audit_log = ROOT / "audit_log"
    if audit_log.is_dir():
        for p in audit_log.rglob("*"):
            rel = p.resolve().relative_to(ROOT).as_posix()
            targets.add(rel)
            if p.is_dir():
                targets.add(rel.rstrip("/") + "/")

    return targets


def resolve_link(source: str, link: str, targets: set) -> bool:
    if not link:
        return True
    if link.startswith(("http://", "https://", "#", "mailto:")):
        return True

    if link.startswith("/"):
        # Absolute from repository root
        target = link[1:]
    else:
        src_dir = (ROOT / Path(source).parent).resolve()
        target = (src_dir / link).resolve().relative_to(ROOT).as_posix()

    # Drop fragment
    target = target.split("#")[0]

    if not target:
        return True

    # Exact file match
    if target in targets:
        return True

    # Directory match (link to a directory)
    if target.rstrip("/") + "/" in targets:
        return True

    # Allow trailing slash on directory links
    if target.endswith("/") and target.rstrip("/") in targets:
        return True

    return False


def main():
    targets = collect_targets()
    link_re = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
    broken = []
    for md in sorted(DOCS_ROOT.rglob("*.md")):
        rel = md.resolve().relative_to(ROOT).as_posix()
        text = md.read_text(encoding="utf-8")
        for _, link in link_re.findall(text):
            if not resolve_link(rel, link, targets):
                broken.append((rel, link))

    if broken:
        print(f"Found {len(broken)} broken internal links:")
        for src, link in broken:
            print(f"  {src} -> {link}")
        return 1
    print("No broken internal links found.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
