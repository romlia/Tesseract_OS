# Protocole de test local non personnel — ŧøß Node v1

Auteur : **Manus AI**  
Date : 15 mai 2026  
Statut : protocole v1, local-first, non transactionnel, non identitaire, réversible.

## 1. Objet du test

Ce test vérifie uniquement la chaîne minimale de **ŧøß Node v1** : un geste humain explicite déclenche l’écriture d’un événement fictif dans un journal local en texte clair. Le test ne prouve pas encore une interface physique, un NFC, un OS ou une incarnation complète. Il valide seulement que le noyau de journalisation peut rester lisible, borné et réversible.

> Le test est réussi si un seul événement fictif est inscrit localement, sans réseau, sans donnée personnelle, sans transaction et sans action cachée.

## 2. Fichiers concernés

| Fichier | Rôle | Statut |
|---|---|---|
| `logger_sync_link_time.py` | Script minimal de génération d’événement fictif | Exécutable localement |
| `logs/sync_link_time.jsonl` | Journal local produit après exécution | Créé automatiquement |
| `PROTOCOLE_TEST_LOCAL.md` | Présent protocole opérateur | Lecture humaine |

## 3. Contraintes obligatoires

Le test doit respecter cinq contraintes opérationnelles simples. Si l’une d’elles échoue, le test doit être considéré comme **non validé**.

| Contrainte | Critère de validation |
|---|---|
| Zéro réseau | Le script n’importe aucune bibliothèque réseau et ne fait aucun appel externe |
| Texte clair | Le journal est lisible dans un éditeur texte |
| Non personnel | L’événement ne contient aucun nom, email, identifiant réel ou donnée intime |
| Non transactionnel | L’événement ne contient aucune valeur financière, wallet, achat ou token |
| Réversible | Le fichier `logs/sync_link_time.jsonl` peut être supprimé manuellement |

## 4. Commande de test

Depuis le dossier `tos_node_v1`, la commande prévue est :

```bash
python3.11 logger_sync_link_time.py
```

La sortie attendue affiche le chemin du journal écrit et le contenu JSON de l’événement. Le fichier produit doit se trouver ici :

```text
logs/sync_link_time.jsonl
```

## 5. Critères d’acceptation

| Critère | Attendu |
|---|---|
| Création du journal | Le fichier `logs/sync_link_time.jsonl` existe après exécution |
| Format | Chaque ligne est un objet JSON valide |
| Données personnelles | `personal_data` vaut `false` |
| Réseau | `network_required` vaut `false` |
| Transaction | `transactional` vaut `false` |
| Réversibilité | `reversible` vaut `true` |
| Garde-fous | Les six verrous sont présents dans `guardrails` |

## 6. Interdits de cette étape

Cette étape ne doit pas ajouter de page distante, de bibliothèque externe, de police hébergée, de CDN, de base de données, de télémétrie, de service cloud, d’identifiant utilisateur ou de mécanisme monétaire. Elle ne doit pas non plus être présentée comme un produit fini.

## 7. Suite logique après validation

Si le test réussit, la prochaine étape sera de préparer le **Paquet 2 — Page locale source/NFC simulé**, mais seulement après inspection du journal produit. Cette page devra être entièrement locale, sans CDN, sans requête sortante, sans analytics et sans collecte personnelle.

