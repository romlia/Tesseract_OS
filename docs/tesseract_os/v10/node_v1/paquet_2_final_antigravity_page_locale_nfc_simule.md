# Paquet 2 final — Page locale source / NFC simulé

Auteur : **Manus AI**  
Destinataire : **Antigravity**  
Mode : copier-coller manuel par l’utilisateur  
Statut : paquet final après accueil de Perplexity et qualification NotebookLM/Drive, borné, non transactionnel, sans secret, sans exécution automatique.

## Message à copier-coller dans Antigravity

Bonjour Antigravity,

Avant transmission de la phase 2, Manus a accueilli **Perplexity** dans T•OI comme **source externe encadrée**, puis a examiné le NotebookLM et le dossier Google Drive fournis par l’utilisateur. La règle de souveraineté reste inchangée :

> L’externe conseille ; le local demeure souverain.

Perplexity, NotebookLM et Google Drive peuvent éclairer la conception, mais aucun de ces services ne doit devenir un composant runtime du prototype local. NotebookLM est traité comme **mémoire documentaire** ; le dossier Drive est traité comme **matière d’audit et inspiration de design** ; aucun des deux ne doit être requis pour ouvrir ou comprendre la page locale.

## 1. Périmètre final de la phase

La page locale doit simuler ce qu’un utilisateur verrait après un geste NFC symbolique ou après l’ouverture d’une source locale. Cette phase ne doit pas intégrer de vrai lecteur NFC, pas de serveur obligatoire, pas de cloud, pas de base de données, pas d’analytics, pas de bibliothèque distante, pas de Perplexity runtime, pas de NotebookLM runtime, pas de Drive runtime et pas de JavaScript.

> Objectif : produire une page statique locale, auditable, lisible hors ligne, qui affiche une source, les garde-fous, le statut du prototype et l’explication du geste NFC simulé.

## 2. Note spécifique NotebookLM / Drive

Manus a repéré dans le Drive une structure web plus ambitieuse de type React/Vite. Cette structure peut être utile à une phase ultérieure de site documentaire, mais elle est **hors périmètre** pour ŧøß Node v1 phase 2. L’audit doit donc refuser toute reprise directe de React, Vite, Tailwind, composants interactifs, images CloudFront, polices Google, scripts analytics, liens automatiques ou dépendances issues du Drive.

En revanche, le fichier `sources.ts` du Drive fournit une taxonomie utile : `guardRailsV10`, `Sync_Link_Time` et `nfcNodeCases`. Ces éléments peuvent être repris conceptuellement comme texte affiché, sans import, sans compilation et sans dépendance au Drive.

## 3. Garde-fous à vérifier

| Verrou | Exigence pour la page locale |
|---|---|
| ¢ø | Aucune valeur monétaire, aucun prix, aucune promesse de rendement |
| ¬monnaie | Aucun wallet, token, paiement, dépôt ou mécanisme financier |
| ¬identité | Aucun nom réel, email, identifiant utilisateur, empreinte navigateur ou donnée intime |
| ¬pseudo-science | La simulation NFC reste explicitement une simulation ; aucun effet physique non prouvé n’est revendiqué |
| ¬exécution_cachée | Aucun tracking, aucun appel réseau, aucune lecture silencieuse de fichiers, aucune action invisible |
| ¬capture | Aucun CDN, police distante, framework hébergé, analytics, API ou service externe gouvernant le noyau |
| ¬dépendance_runtime_externe | Aucune API externe, y compris Perplexity, NotebookLM ou Drive, ne doit être nécessaire au fonctionnement local |

## 4. Artefact souhaité

La forme demandée est stricte : **un seul fichier `index.html` autonome**. Il peut contenir du CSS local intégré dans une balise `<style>`, mais ne doit contenir aucun JavaScript.

| Option | Décision | Commentaire |
|---|---|---|
| `index.html` autonome sans JS | Acceptée et préférée | Un seul fichier, audit humain direct, zéro dépendance |
| CSS intégré dans `<style>` | Accepté | Local, lisible, sans import |
| `style.css` séparé | À éviter | Acceptable seulement si Antigravity juge la séparation indispensable |
| JavaScript | Refusé pour cette phase | La simulation informative n’en a pas besoin |
| React/Vite/Tailwind/runtime framework | Refusé | Risque de dépendances implicites et complexité inutile |
| CDN / polices distantes / images distantes / analytics | Refusé | Violation directe de `¬capture` ou `¬exécution_cachée` |
| Perplexity, NotebookLM, Drive ou autre API dans la page | Refusé | Les services externes restent outils de conception, pas composants runtime |

## 5. Contenu minimal attendu de la page

La page doit afficher clairement les éléments suivants :

| Section | Contenu attendu |
|---|---|
| Titre | `ŧøß Node v1 — Source locale / NFC simulé` |
| Statut | `Prototype local, non transactionnel, non identitaire, réversible` |
| Geste simulé | `Cette page simule l’ouverture d’une source locale après un geste NFC. Aucun lecteur NFC réel n’est utilisé.` |
| Source locale | Une référence locale fictive, par exemple `source://local/tos-node-v1/prototype-test` |
| Garde-fous | Affichage visible des verrous : `¢ø`, `¬monnaie`, `¬identité`, `¬pseudo-science`, `¬exécution_cachée`, `¬capture`, `¬dépendance_runtime_externe` |
| Journal | Mention du journal local validé : `logs/sync_link_time.jsonl`, sans lecture automatique |
| NFC cases | Mention informative : lecture NFC locale simulée autorisée seulement si non financière ; paiement réel restreint et non activé |
| Limites | `Simulation locale ; aucun lecteur NFC réel ; aucun réseau ; aucune donnée personnelle ; aucune transaction.` |
| Preuve de non-action | Une section expliquant que la page n’envoie rien, ne stocke rien, ne lit rien automatiquement et ne dépend d’aucun service externe |

## 6. Ta mission d’audit

Tu dois répondre uniquement sous forme d’audit. Si tu proposes un squelette HTML, il doit être minimal, statique, local, sans JavaScript, sans CDN, sans import, sans analytics, sans formulaire, sans stockage navigateur et sans appel réseau. Tu dois refuser toute complexification inutile.

Merci de répondre dans ce format exact :

```markdown
# Audit Antigravity — Paquet 2 final Page locale source / NFC simulé

## 1. Verdict synthétique
OK / OK sous réserve / À bloquer

## 2. Cohérence avec les garde-fous
| Verrou | Statut | Commentaire |
|---|---|---|
| ¢ø |  |  |
| ¬monnaie |  |  |
| ¬identité |  |  |
| ¬pseudo-science |  |  |
| ¬exécution_cachée |  |  |
| ¬capture |  |  |
| ¬dépendance_runtime_externe |  |  |

## 3. Risques détectés
Décrire les risques liés à une page locale, notamment les requêtes sortantes, ressources implicites, fausse lecture NFC, confusion produit/prototype, stockage navigateur, collecte involontaire, ou reprise excessive du prototype React/Vite observé dans le Drive.

## 4. Forme d’artefact recommandée
Confirmer ou refuser la forme : `index.html` autonome sans JavaScript.

## 5. Contraintes minimales avant intégration
Lister seulement ce qui est nécessaire pour que Manus puisse créer la page locale sans violer les garde-fous.

## 6. Proposition de squelette HTML minimal facultative
Si utile, proposer un squelette `index.html` autonome, sans JavaScript, sans CDN, sans import externe, sans formulaire, sans stockage navigateur et sans appel réseau.

## 7. Questions bloquantes éventuelles
Lister uniquement les questions nécessaires avant passage à l’étape suivante.

## 8. Recommandation finale
Dire si Manus peut créer la page locale source / NFC simulé, ou s’il faut bloquer/corriger d’abord.
```

## 7. Contraintes finales

Tu ne dois pas recommander de package manager, de framework, de déploiement, de serveur distant, d’appel API, de télémétrie, de QR code externe, de wallet, d’identification utilisateur, de JavaScript, de lecture NFC réelle ou de reprise directe du projet React/Vite Drive à cette étape. Le but est de garder le prototype dans son état le plus simple, local et auditable.

Fin du paquet.

## Instruction de retour vers Manus

Après avoir obtenu la réponse d’Antigravity, l’utilisateur doit la copier intégralement dans Manus, sans la modifier. Manus réalisera alors l’audit de second niveau avant toute création ou intégration de la page locale.
