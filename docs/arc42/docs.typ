#import "template.typ": *
#import "@preview/codelst:2.0.2": sourcecode

#show: project.with(
  subject: "LOG430",
  title: "BrokerX - Documentation d'Architecture",
  authors: (
    "Thevenet, Louis",
  ),
  date: "Automne 2025",
  subtitle: "Basé sur le modèle arc42",
  toc: true,
)
= Avant-propos

Les figures sont trouvables au format SVG dans le répertoire `docs/arc42/`.

= Introduction et Objectifs

== Panorama des exigences

L'application « BrokerX » est dotée d'une interface web faisant appel à une passerelle API sécurisée faisant elle-même appel à différents micro-services. Elle permet aux particuliers et petites entreprises d'accéder à une plateforme de courtage moderne. Les clients peuvent ainsi :

- Gérer un portefeuille personnel (solde, historique),
- Passer des ordres d'achat/vente d'instruments financiers (actions),
- Consulter les données de marché en temps réel.

== Objectifs qualité

#figure(
  table(
    columns: 3,
    align: (left, left, left),
    table.header([*Priorité*], [*Objectif qualité*], [*Scénario*]),
    [1], [*Disponibilité*], [95,5 %],
    [2],
    [*Performance*],
    [Latence P95 (ordre → ACK) ≤ 250 ms & Débit ≥ 800 ordres/s],

    [3], [*Observabilité*], [Logs + métriques (4 Golden Signals)],
  ),
  caption: [Objectifs qualité],
)

== Parties prenantes (Stakeholders)
- *Clients* : utilisateurs via interface web.
- *Opérations Back-Office* : gestion des règlements, supervision.
- *Risque* : surveillance pré- et post-trade.
- *Fournisseurs de données de marché* : cotations en temps réel.
- *Bourses externes* : simulateurs de marché pour routage d'ordres.

= Contraintes d'architecture

#figure(
  table(
    columns: 2,
    align: (left, left),
    table.header([*Contrainte*], [*Description*]),
    [*Technologie*],
    [Les plateformes/langages serveur privilégiées : Java, C\#, Go, Rust ou C++],
  ),
  caption: [Contraintes d'architecture],
)


= Portée et contexte du système

== Contexte métier

#figure(image("activity.svg", width: 80%), caption: [Activity])

Le système permet aux utilisateurs de :

- Gérer leur compte utilisateur
- Approvisionner et retirer des fonds
- Passer des ordres d'achat/vente
- Consulter l'état des ordres et le portefeuille
- Consulter les données du marché

=== Priorisation MoSCoW

- Must Have
  - UC-01 — Inscription & Vérification d'identité [FAIT]
  - UC-03 — Approvisionnement du portefeuille (dépôt virtuel) [FAIT]
  - UC-05 — Placement d'un ordre (marché/limite) avec contrôles pré-trade [FAIT]
- Should Have
  - UC-07 — Appariement interne & Exécution (matching) [NON FAIT]
  - UC-06 — Modification / Annulation d'un ordre [FAIT]
- Could Have
  - UC-08 — Confirmation d'exécution et Notifications [NON FAIT]
  - UC-02 — Authentification & MFA [FAIT]
  - UC-04 — Abonnement aux données de marché [NON FAIT]
- Won't Have

=== UC-01 — Inscription & Vérification d'identité

Ce cas d'utilisation est le point d'entrée d'un nouvel utilisateur sur la plateforme BrokerX. L'inscription comprend les étapes suivantes :

- Le client remplit un formulaire d'inscription avec des informations de base (email, mot de passe, etc).
- Le système crée son compte utilisateur avec le statut "Non vérifié"
- Il sera vérifié à la première connexion, et en attendant il ne pourra pas effectuer d'actions.

=== UC-03 — Approvisionnement du portefeuille (dépôt virtuel)

Ce cas d'utilisation permet à un utilisateur de déposer des fonds sur son compte BrokerX. Le processus comprend les étapes suivantes :

- L'utilisateur se connecte à son compte.
- Il navigue vers la section "Approvisionnement".
- Il saisit le montant à déposer.
- Le système valide le montant (positif, limites).
- Le système crédite le portefeuille de l'utilisateur avec le montant déposé. (dépôt virtuel)
- Le système affiche confirmation

=== UC-05 — Placement d'un ordre (marché/limite) avec contrôles pré-trade

Ce cas d'utilisation permet à un utilisateur de passer un ordre d'achat ou de vente d'instruments financiers. Le processus comprend les étapes suivantes :

- L'utilisateur se connecte à son compte.
- Il navigue vers la section "Passer un ordre".
- Il remplit le formulaire d'ordre avec les détails (type, quantité, prix, etc).
- Le système valide le formulaire (champs obligatoires, formats).
- La demande d'ordre est envoyée
- Le système effectue des contrôles pré-trade (solvabilité, limites de risque).
- L'utilisateur peut consulter l'état de son ordre en direct
- Le système traite l'ordre en arrière-plan et met à jour son état (exécuté, rejeté, annulé).

=== UC-02 — Authentification & MFA
Ce cas d'utilisation permet à un utilisateur de se connecter de manière sécurisée à son compte BrokerX en utilisant l'authentification multi-facteurs (MFA). Le processus comprend les étapes suivantes :

- L'utilisateur accède à la page de connexion.
- Il saisit son email et son mot de passe.
- Le système valide les informations d'identification.
- Si les informations sont correctes, le système envoie un code OTP par email à l'utilisateur.
- L'utilisateur saisit le code OTP reçu.
- Le système valide le code OTP.
- Le système transmfet un token JWT pour les sessions authentifiées.

=== UC-06 — Modification / Annulation d'un ordre

Ce cas d'utilisation permet à un utilisateur de modifier ou d'annuler un ordre en attente. Pour chaque ordre en statut "Pending", l'utilisateur peut l'annuler ou le modifier (quantité, prix limite et type). Le système valide la demande et met à jour l'état de l'ordre en conséquence. (En réalité les ordres sont traités très rapidement donc cette fonctionnalité est difficile à utiliser en l'état.)

== Contexte technique

- *Applications clientes* : Postman, frontend web
- *Couche API*: Axum (Rust) pour les endpoints RESTful
- *Couche base de données* : PostgreSQL
- *API Gateway* : Axum (Rust)
- *Communication* : Requêtes HTTP/HTTPS entre clients, gateway et microservices

= Stratégie de solution

// TODO: compléter ça

#figure(
  table(
    columns: 2,
    align: (left, left),
    table.header([*Problème*], [*Approche de solution*]),
    [*Performance*], [Utilisation du langage Rust],
    [*Gestion des données*],
    [Base relationnelle (*PostgreSQL*), cache en mémoire],

    [*Observabilité*], [Métriques Promethus et Dashboards Grafana],
    [*Rapports statistiques*], [],
  ),
  caption: [Stratégie de solution],
)

= Vue des blocs de construction

== Vue d'ensemble du système (Niveau 1)

#figure(
  image("level1_context.svg", width: 80%),
  caption: [Vue d'ensemble du système],
)

Le système BrokerX suit une architecture restAPI avec passerelle API et microservices :

+ La passerelle API est le point d'entrée unique pour les clients.
+ La passerelle API orchestre l'authentification via MFA ($->$ token JWT)
+ Le service de traitement des ordres communique également avec les différents micro-services, il agit comme un orchestrateur pour le flux de traitement des ordres.

+ Les différents services sont :
  - Service Utilisateurs : gestion des comptes et portefeuilles utilisateurs
  - Service Ordres : gestion des ordres en mémoire persistente
  - Service Données de Marché : récupération des cotations en temps réel, execution des ordres sur le marché
  - Service de traitement des ordres: contrôles pre/post-trades, orchestration du traitement des ordres


== Architecture du domaine (Niveau 2)

#figure(
  image("level2_domain.svg", width: 80%),
  caption: [Architecture du domaine],
)

Le diagramme détaille l'architecture microservices avec une séparation claire entre les couches métier et infrastructure (pattern Ports & Adapters). L'architecture se compose de :

=== Composants principaux

- *API Gateway* : Point d'entrée unique assurant le routage, l'authentification JWT/MFA via `MfaService`

- *UserService* : Gestion des comptes utilisateurs, authentification, solde et positions (holdings)

- *OrderService* : Gestion du cycle de vie des ordres (CRUD, exécution, annulation)

- *MarketDataService* : Données de marché en temps réel, historique, et simulation d'exécution

- *OrderOrchestrator* : Traitement asynchrone des ordres via file d'attente (validation pré-trade via `TradeValidator` → exécution → mise à jour post-trade)

=== Architecture des données

Chaque microservice possède son propre repository (pattern Repository) pour l'isolation des domaines :
- `UserRepository` → entités `User` et `Holding`
- `OrderRepository` → entité `Order`
- `MarketDataRepository` → entité `Stock`

== Détail du traitement des ordres

#figure(
  image("order_processing_detail.svg", width: 80%),
  caption: [Détail du traitement des ordres],
)

Le diagramme de séquence illustre le workflow complet de traitement d'un ordre, décomposé en deux phases distinctes :

=== Phase 1 : Acceptation synchrone (API Gateway)

+ L'utilisateur soumet un ordre via `POST /v1/orders` avec authentification JWT
+ L'API Gateway valide le JWT et les champs de la requête
+ L'ordre est créé dans `OrderService` avec le statut `PENDING`
+ L'ordre est transmis à `OrderOrchestrator` qui le place dans une file d'attente
+ Réponse immédiate `202 Accepted` à l'utilisateur avec l'`order_id`

L'utilisateur peut ensuite consulter l'état de son ordre en temps réel via polling.

=== Phase 2 : Traitement asynchrone (Order Orchestrator)

L'orchestrateur traite les ordres en arrière-plan avec contrôle de concurrence :

==== Validation pré-trade (TradeValidator)

- Récupération des données utilisateur (solde, holdings) via `UserService`
- Récupération du prix de marché via `MarketDataService`
- Vérifications : solvabilité, limites, bandes de prix
- Résultat : ordre accepté ou rejeté

==== Exécution (si validation réussie)

- Simulation de l'exécution sur le marché via `MarketDataService`
- Mise à jour du statut de l'ordre (`FILLED` ou `REJECTED`)

==== Règlement post-trade (si ordre exécuté)

- Mise à jour du solde : retrait (achat) ou crédit (vente) via `UserService`
- Mise à jour des positions (holdings) dans `UserService`
- Enregistrement des métriques

L'utilisateur récupère finalement l'état final de son ordre via `GET /v1/orders/:id`.

== Vue d'exécution

=== Parcours utilisateur complet

#figure(
  image("complete_user_journey.svg", width: 80%),
  caption: [Parcours utilisateur complet],
)

Ce diagramme de séquence illustre l'ensemble des interactions utilisateur dans le système BrokerX, de l'inscription initiale jusqu'à la gestion complète du portefeuille. Il couvre tous les cas d'usage implémentés :

=== Flux principaux documentés

+ *Inscription et vérification d'identité (UC-01)*

  - Création de compte avec validation email
  - Processus de vérification par code à 6 chiffres
  - Activation du compte utilisateur

+ *Authentification et MFA (UC-02)*

  - Connexion sécurisée avec email/mot de passe
  - Authentification multi-facteurs obligatoire
  - Génération et validation de tokens JWT

+ *Approvisionnement du portefeuille (UC-03)*

  - Dépôt virtuel de fonds

+ *Placement d'ordres avec contrôles pré-trade (UC-05)*

  - Validation des formulaires d'ordre
  - Contrôles de risque et de solvabilité
  - Traitement asynchrone par le service d'ordres

== Vue de déploiement

#figure(image("deployment_view.svg", width: 80%), caption: [Vue de déploiement])

L'architecture de déploiement utilise *Docker Compose* pour orchestrer l'ensemble des composants en conteneurs. La stack complète comprend :

=== Couche Frontend
- *WebApp* (port 8000) : Interface web statique servie via serveur HTTP

=== Couche API
- *API Gateway* (port 8080) : Point d'entrée unique pour toutes les requêtes clients

=== Couche Microservices
- *User Service* (port 3001) : Gestion des utilisateurs et authentification
- *Order Service* (port 3002) : Gestion des ordres
- *Market Data Service* (port 3003) : Données de marché et simulation d'exécution
- *Order Orchestrator* (port 3004) : Traitement asynchrone des ordres

=== Couche Données
Chaque service possède sa propre base de données PostgreSQL 15 pour l'isolation :
- *User DB* (port 5432) : Données utilisateurs et holdings
- *Order DB* (port 5433) : Ordres avec optimisations pour hautes performances
- *Market Data DB* (port 5434) : Instruments financiers et historique des prix

=== Monitoring
- *Prometheus* (port 9090) : Collecte des métriques via endpoints `/metrics`
- *Grafana* (port 3000) : Visualisation des métriques et dashboards
- *K6* : Des scénarios de tests K6 sont disponibles

=== Réseau et Communication
- Tous les services communiquent via un réseau Docker privé `brokerx-network`
- Communication inter-services en HTTP/REST
- Healthchecks pour garantir l'ordre de démarrage des dépendances

= Concepts transversaux

== Décisions d'architecture

Les décisions architecturales majeures sont documentées sous forme d'ADR (Architecture Decision Records) dans le répertoire `docs/adr/`.

=== ADR documentées

1. *ADR 001* — Adoption de l'architecture hexagonale (ports et adaptateurs) (obsolète)
2. *ADR 002* — Choix de la technologie web (Axum)
3. *ADR 003* — Adoption des UUIDs comme identifiants
4. *ADR 004* — Choix de la technologie de logs (tracing)
5. *ADR 005* — Implémentation du traitement des ordres (machine d'états, pipeline asynchrone)
6. *ADR 006* — Implémentation d'une queue d'ordres à traiter
7. *ADR 007* — Architecture microservices avec API Gateway (5 services distincts)
8. *ADR 008* — Authentification JWT avec MFA (multi-factor authentication)
9. *ADR 009* — Stratégie de caching multi-niveaux (4 caches, batching des écritures)

Chaque ADR décrit le contexte, les options considérées, la décision retenue et ses conséquences.


== Exigences qualité

BrokerX est capable de gérer $>=$ 1800 ordres/s en pic. La latence P95 sur l'endpoint `/v1/orders` est garantie grâce à la queue asynchrone dans l'orchestrateur d'ordres. Même lors de pics de charge et de perte de disponibilité sur certains services, les utilisateurs peuvent toujours passer des ordres qui seront traités en différé.
#figure(caption: "Ordres traités par seconde et taille de la queue", image(
  "./order_per_s.png",
))

On voit que la queue d'ordres monte en pic lors des charges élevées, mais les ordres sont toujours acceptés rapidement par l'API Gateway.
#figure(caption: "Latence P95 sur l'endpoint /v1/orders", image(
  "./p95_orders.png",
))

Le premier service à tomber est celui chargé de mettre à jour les soldes et portefeuilles des utilisateurs.

#figure(caption: "Latence sur tous les endpoints", image("./latency.png"))

// == Risques et dettes techniques

= Glossaire

#figure(
  table(
    columns: 2,
    align: (left, left),
    table.header([*Terme*], [*Définition*]),
    [Ordre], [Instruction d'achat ou de vente d'un instrument financier],
    [Portefeuille], [Ensemble des actifs financiers détenus par un utilisateur],
    [MFA], [Authentification multi-facteurs],
    [OTP], [One-Time Password, mot de passe à usage unique],
    [JWT], [JSON Web Token, format de token sécurisé pour l'authentification],
  ),
  caption: [Glossaire],
)
