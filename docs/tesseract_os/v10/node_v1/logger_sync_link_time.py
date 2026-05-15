#!/usr/bin/env python3
"""
ŧøß Node v1 — Logger Sync_Link_Time minimal

Objet : générer un événement fictif non personnel dans un journal local JSONL.
Contraintes : aucun réseau, aucune donnée personnelle, aucun service externe, texte clair.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent
LOG_DIR = BASE_DIR / "logs"
LOG_FILE = LOG_DIR / "sync_link_time.jsonl"

GUARDRAILS = [
    "¢ø",
    "¬monnaie",
    "¬identité",
    "¬pseudo-science",
    "¬exécution_cachée",
    "¬capture",
]


def build_event() -> dict:
    """Construit un événement de test volontairement fictif et non personnel."""
    return {
        "sync_link_time": datetime.now(timezone.utc).isoformat(),
        "event_type": "prototype_test",
        "source": "local_manual_test",
        "personal_data": False,
        "network_required": False,
        "transactional": False,
        "reversible": True,
        "guardrails": GUARDRAILS,
        "note": "Événement fictif non personnel pour test ŧøß Node v1.",
    }


def append_event(event: dict) -> Path:
    """Ajoute l’événement à un journal local en texte clair JSONL."""
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    with LOG_FILE.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n")
    return LOG_FILE


def main() -> None:
    event = build_event()
    path = append_event(event)
    print("ŧøß Node v1 — test local non personnel")
    print(f"Journal écrit : {path}")
    print("Événement :")
    print(json.dumps(event, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
