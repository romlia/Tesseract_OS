# Rapport de test local — ŧøß Node v1

Auteur : **Manus AI**  
Date : 15 mai 2026  
Statut : test local non personnel exécuté avec succès.

## 1. Résultat synthétique

Le test local non personnel de **ŧøß Node v1** a été exécuté avec succès dans l’environnement de travail. Le script minimal `logger_sync_link_time.py` a généré un événement fictif et l’a inscrit dans un journal local au format **JSON Lines**.

> Verdict Manus : **test local v1 validé**. Le prototype produit une trace locale, lisible, non personnelle, non transactionnelle et réversible.

## 2. Commande exécutée

```bash
cd /home/ubuntu/tos_node_v1 && python3.11 logger_sync_link_time.py
```

La commande a produit un fichier de journalisation local :

```text
/home/ubuntu/tos_node_v1/logs/sync_link_time.jsonl
```

## 3. Événement généré

L’événement inscrit est le suivant :

```json
{"sync_link_time":"2026-05-15T16:25:57.663883+00:00","event_type":"prototype_test","source":"local_manual_test","personal_data":false,"network_required":false,"transactional":false,"reversible":true,"guardrails":["¢ø","¬monnaie","¬identité","¬pseudo-science","¬exécution_cachée","¬capture"],"note":"Événement fictif non personnel pour test ŧøß Node v1."}
```

## 4. Vérification des critères

| Critère | Attendu | Observé | Statut |
|---|---|---|---|
| Création du journal | Le fichier `.jsonl` existe | `logs/sync_link_time.jsonl` créé | Validé |
| Texte clair | Lecture directe possible | Ligne JSON lisible | Validé |
| Données personnelles | Aucune donnée personnelle | `personal_data:false` | Validé |
| Réseau | Aucun réseau requis | `network_required:false` | Validé |
| Transaction | Aucun mécanisme financier | `transactional:false` | Validé |
| Réversibilité | Suppression possible | `reversible:true` | Validé |
| Garde-fous | Six verrous présents | Six verrous inscrits | Validé |

## 5. Limites du test

Ce test ne valide pas encore une interface NFC, une page locale enrichie, un boîtier physique ou une intégration Raspberry Pi. Il valide uniquement le noyau minimal de journalisation locale. Aucune conclusion ne doit être étendue au-delà de ce périmètre.

## 6. Décision de passage

La prochaine étape autorisée est le **Paquet 2 — Page locale source / NFC simulé**. Cette étape devra rester entièrement locale, sans CDN, sans police distante, sans analytics, sans appel API et sans collecte personnelle.

