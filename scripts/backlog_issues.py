#!/usr/bin/env python3
"""Create GitHub issues from BACKLOG.md.

Dry run by default: prints what it would do and calls nothing.
With --apply it creates the labels, creates one issue per open backlog item,
and writes the issue number back into BACKLOG.md (`PROTO-1` -> `PROTO-1 (#12)`).

Skipped: items marked [x], items that already have `(#N)`, and items whose ID
already starts the title of an existing issue (open or closed), so re-running is safe.

Usage:
  scripts/backlog_issues.py                 # dry run
  scripts/backlog_issues.py --apply         # create labels + issues
  scripts/backlog_issues.py --only PROTO-1  # limit to some IDs (repeatable)
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

DEFAULT_REPO = "mityakoval/daikon"  # explicit: the checkout also has a codecrafters remote
BACKLOG = Path(__file__).resolve().parent.parent / "BACKLOG.md"

ID = r"[A-Z]+-\d+"
HEADING = re.compile(rf"^### \[( |x)\] ({ID})( \(#\d+\))? (.+)$")
CHECKLIST = re.compile(rf"^- \[( |x)\] ({ID})( \(#\d+\))? (.+?)(?: \((.+)\))?$")
LABEL = re.compile(r"`([^`]+)`")

LABEL_COLORS = {
    "p0": "b60205", "p1": "d93f0b", "p2": "fbca04", "p3": "c2e0c6",
    "bug": "d73a4a", "refactor": "5319e7", "feature": "a2eeef",
    "test": "0e8a16", "chore": "ededed", "docs": "0075ca",
    "client-crash": "000000", "redis-compat": "f9d0c4",
}
PREFIX_COLORS = {"area:": "1d76db", "size:": "bfdadc"}


def color(label):
    if label in LABEL_COLORS:
        return LABEL_COLORS[label]
    for prefix, c in PREFIX_COLORS.items():
        if label.startswith(prefix):
            return c
    return "cccccc"


def parse(text):
    """Return open items as dicts: id, title, labels, body."""
    lines = text.splitlines()
    items = []
    i = 0
    while i < len(lines):
        line = lines[i]

        m = HEADING.match(line)
        if m:
            done, item_id, issue_ref, title = m.groups()
            j = i + 1
            while j < len(lines) and not (lines[j].startswith(("### ", "## ", "---"))
                                          or CHECKLIST.match(lines[j])):
                j += 1
            section = lines[i + 1:j]
            labels = []
            if section and section[0].startswith("Labels:"):
                labels = LABEL.findall(section[0])
                section = section[1:]
            if done == " " and not issue_ref:
                items.append({"id": item_id, "title": title, "labels": labels,
                              "body": "\n".join(section).strip()})
            i = j
            continue

        m = CHECKLIST.match(line)
        if m:
            done, item_id, issue_ref, title, extra = m.groups()
            labels = ["feature", "area:commands"]
            notes = []
            for part in (extra or "").split(", "):
                found = LABEL.fullmatch(part.strip())
                if found:
                    labels.append(found.group(1))
                elif part.strip():
                    notes.append(part.strip())
            body = "Command from the roadmap. Behavior and edge cases: REDIS_NOTES.md §4 / §5."
            if notes:
                body += "\n\n" + "\n".join(f"- {n[0].upper()}{n[1:]}" for n in notes)
            if done == " " and not issue_ref:
                items.append({"id": item_id, "title": title, "labels": labels, "body": body})
        i += 1
    return items


def gh(*args, stdin=None):
    result = subprocess.run(["gh", *args], input=stdin, capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"gh {' '.join(args[:2])} failed:\n{result.stderr}")
    return result.stdout


def existing_ids(repo):
    out = gh("issue", "list", "--repo", repo, "--state", "all", "--limit", "1000",
             "--json", "number,title")
    ids = {}
    for issue in json.loads(out):
        m = re.match(rf"({ID}):", issue["title"])
        if m:
            ids[m.group(1)] = issue["number"]
    return ids


def write_back(text, numbers):
    for item_id, number in numbers.items():
        # Only the heading/checklist line, not mentions like "Depends on: PROTO-1".
        text = re.sub(rf"^((?:###|-) \[ \] {re.escape(item_id)})(?= )",
                      rf"\1 (#{number})", text, count=1, flags=re.M)
    return text


def self_check():
    sample = (
        "### [ ] A-1 First\nLabels: `p0`, `bug`\nBody line\n\n---\n"
        "### [x] A-2 Done\nLabels: `p1`\n"
        "### [ ] A-3 (#7) Has issue\nLabels: `p1`\n"
        "- [ ] B-1 `CMD` (`p2`, `size:S`, depends on A-1)\n"
    )
    items = parse(sample)
    assert [i["id"] for i in items] == ["A-1", "B-1"], items
    assert items[0]["labels"] == ["p0", "bug"] and items[0]["body"] == "Body line"
    assert items[1]["labels"] == ["feature", "area:commands", "p2", "size:S"]
    assert "Depends on A-1" in items[1]["body"]
    out = write_back(sample + "Depends on: A-1\n", {"A-1": 5, "B-1": 6})
    assert "### [ ] A-1 (#5) First" in out and "- [ ] B-1 (#6) `CMD`" in out
    assert "Depends on: A-1\n" in out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true", help="actually create labels and issues")
    ap.add_argument("--repo", default=DEFAULT_REPO)
    ap.add_argument("--only", action="append", default=[], metavar="ID")
    args = ap.parse_args()

    self_check()
    text = BACKLOG.read_text()
    items = parse(text)
    if args.only:
        items = [i for i in items if i["id"] in args.only]

    if not args.apply:
        for it in items:
            print(f"{it['id']}: {it['title']}\n    labels: {', '.join(it['labels'])}")
        print(f"\n{len(items)} issue(s) would be created in {args.repo}. Re-run with --apply.")
        return

    for label in sorted({l for it in items for l in it["labels"]}):
        gh("label", "create", label, "--repo", args.repo, "--color", color(label), "--force")

    already = existing_ids(args.repo)
    numbers = {}
    for it in items:
        if it["id"] in already:
            print(f"skip {it['id']}: already #{already[it['id']]}")
            numbers[it["id"]] = already[it["id"]]
            continue
        cmd = ["issue", "create", "--repo", args.repo,
               "--title", f"{it['id']}: {it['title']}", "--body-file", "-"]
        for label in it["labels"]:
            cmd += ["--label", label]
        body = f"{it['body']}\n\n_From BACKLOG.md, item {it['id']}._"
        url = gh(*cmd, stdin=body).strip()
        numbers[it["id"]] = int(url.rsplit("/", 1)[1])
        print(f"created {it['id']}: {url}")
        # Write back after each issue so a failure halfway doesn't lose numbers.
        text = write_back(text, {it["id"]: numbers[it["id"]]})
        BACKLOG.write_text(text)

    text = write_back(text, numbers)
    BACKLOG.write_text(text)


if __name__ == "__main__":
    main()
