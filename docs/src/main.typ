#import "template.typ": *
#import "@preview/fletcher:0.5.8"
#import "@preview/pintorita:0.1.4"
#show raw.where(lang: "pintora"): it => pintorita.render(it.text)

#show: project.with(
  subject: "Architecture Logicielle",
  title: "Phase 1",
  authors: (
    "THEVENET Louis",
  ),
  date: "",
  subtitle: "",
  toc: true,
)

= Analyse métier & DDD
- Clarifier le périmètre initial & cas d’utilisation Must
  - Livrables : UC textuels (scénarios principal/alternatifs), priorisation MoSCoW.
  - CA : au moins 3 UC Must entièrement décrits et validés.

- Cartographier le domaine (DDD)
  - Livrables : bounded contexts, ubiquitous language, diagramme de contexte, esquisse du modèle de domaine.
  - CA : glossaire validé, entités/agrégats identifiés pour les UC Must.


== Priorisation MoSCoW
- Must Have
  - UC-01 — Inscription & Vérification d’identité
  - UC-03 — Approvisionnement du portefeuille (dépôt virtuel)
  - UC-05 — Placement d’un ordre (marché/limite) avec contrôles pré-trade
  - UC-07 — Appariement interne & Exécution (matching)
- Should Have
  - UC-06 — Modification / Annulation d’un ordre
- Could Have
  - UC-08 — Confirmation d’exécution & Notifications
  - UC-02 — Authentification & MFA
  - UC-04 — Abonnement aux données de marché
- Won't Have
#pagebreak()
== Use Cases Must
=== UC-01 — Inscription & Vérification d’identité
/ Objectif: ​Permettre à un nouvel utilisateur de créer un compte sur la plateforme en fournissant ses informations personnelles, de vérifier son identité selon les exigences réglementaires (KYC/AML) et d’activer son accès à la plateforme. Ce cas établit la relation de confiance initiale entre l’utilisateur et BrokerX.
/ Acteur principal: Client​
/ Déclencheur: L’utilisateur souhaite créer un compte.​
/ Pré-conditions: Aucune.​
/ Postconditions (succès): Compte créé en état Pending et changer à Active après validation.​
/ Postconditions (échec): Compte non créé ou marqué Rejected avec raison.
/ Flux principal (succès):
+ Le Client fournit email/téléphone, mot de passe, données personnelles requises (nom, adresse, date de naissance).​
+ Le Système valide le format, crée un compte en Pending, envoie un lien de vérification email/SMS.​
+ Le Client confirme le lien OTP (one-time passwords)/MFA (multi-factor authentication).​
+ Le Système passe le compte à Active et journalise l’audit (horodatage, empreinte des
documents).​
/ Alternatifs / Exceptions​:
- A1. Vérif email non complétée : compte reste Pending (rappel, expiration après X jours).​
- E1. Doublon (email/tel déjà utilisés) : rejet, proposition de récupération de compte.

#pagebreak()
=== UC-03 — Approvisionnement du portefeuille (dépôt virtuel)
/ Objectif: Donner aux utilisateurs la possibilité de créditer leur portefeuille virtuel en effectuant des dépôts simulés, afin de disposer de liquidités nécessaires pour placer des ordres d’achat. Ce cas assure la disponibilité des fonds pour les opérations boursières.
/ Acteur principal: Client​
/ Secondaires: Service Paiement Simulé / Back-Office​
/ Déclencheur: Le Client crédite son solde en monnaie fiduciaire simulée.​
/ Préconditions: Compte Active.​
/ Postconditions (succès): Solde augmenté, écriture comptable ajoutée (journal immuable).​
/ Postconditions (échec): Solde inchangé.
/ Flux principal:
+ Le Client saisit le montant.​
+ Le Système valide limites (min/max, anti-fraude).​
+ Le Système crée une transaction Pending.​
+ Le Service Paiement Simulé répond Settled.​
+ Le Système crédite le portefeuille, journalise et notifie.​
/ Alternatifs / Exceptions​:
- A1. Paiement async : passe Pending, solde crédite à confirmation.​
- E1. Paiement rejeté : état Failed, notification avec motif.​
- E2. Idempotence : si retry reçu avec même idempotency-key, renvoyer le résultat précédent.
#pagebreak()
=== UC-05 — Placement d’un ordre (marché/limite) avec contrôles pré-trade
/ Objectif: Permettre aux clients de soumettre des ordres d’achat ou de vente (marché ou limite), qui seront validés par des contrôles pré-trade et insérés dans le moteur d’appariement. Ce cas constitue le cœur fonctionnel de la plateforme de courtage.
/ Acteur principal: Client​
/ Secondaires: Moteur de Règles Pré-trade, Gestion des Risques, Comptes/Portefeuilles​
/ Déclencheur: Le Client soumet un ordre.​
/ Préconditions: Session valide, portefeuille existant.​
/ Postconditions (succès): Ordre accepté (ACK) et placé dans le carnet interne.​
/ Postconditions (échec): Ordre rejeté avec raison.
/ Flux principal (succès):
+ Le Client renseigne symbole, sens (Achat/Vente), type (Marché/Limite), quantité, prix (si limite), durée (DAY/IOC…).
+ Le Système normalise les données et horodate l’opération (timestamp système en UTC avec millisecondes ou nanosecondes).
+ Contrôles pré-trade
  - Pouvoir d’achat / marge disponible,​
  - Règles de prix (bandes, tick size),​
  - Interdictions (short-sell si non autorisé),​
  - Limites par utilisateur (taille max, notionals),​
  - Sanity checks (quantité > 0, instrument actif).​
+ Si OK, le Système attribue un OrderID, persiste, achemine au Moteur d’appariement interne.

/ Alternatifs / Exceptions​:
- A1. Type Marché : prix non requis, routage immédiat.​
- A2. Durée IOC/FOK : logique spécifique au matching (voir UC-07).​
- E1. Pouvoir d’achat insuffisant : Reject avec motif.​
- E2. Violation bande de prix : Reject.​
- E3. Idempotence : même clientOrderId → renvoyer résultat précédent.
#pagebreak()
=== UC-07 — Appariement interne & Exécution (matching)
/ Objectif: Assurer l’exécution automatique des ordres en interne selon les règles de priorité (prix/temps) en rapprochant acheteurs et vendeurs. Ce cas fournit la mécanique centrale de traitement des transactions sur la plateforme.
/ Acteur principal: Moteur d’appariement interne​
/ Secondaires: Données de Marché, Portefeuilles​
/ Déclencheur: Nouvel ordre arrive dans le carnet.​
/ Préconditions: Carnet maintenu (prix/temps), règles de priorité définies.​
/ Postconditions (succès): Transactions générées (partielles possibles), état d’ordre mis à jour.​
/ Postconditions (échec): Ordre reste Working (pas de contrepartie).
/ Flux principal:
+ Le Moteur insère l’ordre dans le carnet (Buy/Sell).​
+ Il recherche la meilleure contrepartie (price-time priority).​
+ Si match, crée une ou plusieurs exécutions (fills), met à jour quantités.​
+ Émet événements ExecutionReport (Fill/Partial Fill).​
+ Met à jour top-of-book, publie update marché.​
/ Alternatifs / Exceptions:
- A1. Ordre marché sans liquidité: exécution partielle possible, reste non exécuté → voir UC-08 (routage).​
- A2. IOC/FOK : IOC exécute le possible puis annule le reste; FOK exécute tout sinon annule.​
- E1. Incohérence carnet (rare) : déclenche rollback segmentaire et alerte Ops.
#pagebreak()

== Domaines principaux (bounded contexts DDD)
+ Client & Comptes
  - balances, authentification et autorisation, intégrations AML.​
+ Ordres & Appariement
  - Routage broker + moteur d’appariement interne.​
+ Données de marché
  - Quotations, transactions, OHLC (Open, High, Low, Close), diffusion en continu
aux clients.​
+ Portefeuilles & Positions​

== Glossaire
== Diagramme de contexte
#import "@preview/diagraph:0.3.5": raw-render
#raw-render(
  ```dot
  digraph G {
    rankdir=LR   // left-to-right
    { rank=source; brokerX }
    node [shape=box]
    edge [  decorate=true]
    brokerX

    clients [label="Clients"]
    backoffice [label="Opérations Back-Office"]
    conformity [label="Conformité / Risque"]
    marketdata [label="Fournisseurs de données de marché"]
    bourse [label="Bourses externes"]

    clients -> brokerX [label="Créer compte / Placer ordre / Vérifier solde"]
    brokerX -> clients [label="Confirmation / Données de marché / Solde"]

    brokerX -> marketdata [label="S'abonner à"]
    marketdata -> brokerX [label="Flux de données de marché"]

    brokerX -> bourse [label="Envoyer ordres"]
    bourse -> brokerX [label="Confirmations / Exécutions"]

    brokerX -> conformity [label="Surveillance pré et post-trade"]
    conformity -> brokerX [label="Rapports"]

    brokerX -> backoffice [label="Execution des ordres"]
    backoffice -> brokerX [label="Confirmation d'ordre"]
   }
  ```,
)
