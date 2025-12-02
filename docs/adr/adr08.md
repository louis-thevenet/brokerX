# ADR 008 – Event Sourcing pour audit immutable

## Statut

Acceptée

## Contexte

Dans un système de trading, la traçabilité complète des ordres est essentielle pour :

- **Audit réglementaire** : Historique immutable de toutes les transactions
- **Debugging** : Comprendre exactement ce qui s'est passé
- **Reconstitution d'état** : Capacité à recalculer l'état à tout moment

Le modèle CRUD traditionnel ne convient pas car les mises à jour écrasent les valeurs précédentes.

## Décision

Implémentation d'**Event Sourcing** : stocker la séquence d'événements au lieu de l'état actuel.

**Principe** : `État = fold(événements_chronologiques)`

## Architecture

### Table d'événements

```sql
CREATE TABLE order_events (
    sequence_number BIGSERIAL PRIMARY KEY,
    event_id UUID NOT NULL UNIQUE,
    order_id UUID NOT NULL,
    user_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    symbol VARCHAR(10) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL,
    order_type JSONB NOT NULL,
    filled_quantity BIGINT,
    average_price DECIMAL(15, 2),
    occurred_at TIMESTAMPTZ NOT NULL,
    correlation_id UUID NOT NULL,
    causation_id UUID,
    metadata JSONB
);

-- Immutabilité garantie
CREATE RULE no_updates AS ON UPDATE TO order_events DO INSTEAD NOTHING;
CREATE RULE no_deletes AS ON DELETE TO order_events DO INSTEAD NOTHING;
```

### Types d'événements

Les événements métier capturent ce qui s'est passé :

- `OrderCreated` : Nouvel ordre créé
- `OrderValidated` : Validation pré-trade réussie
- `OrderRejected` : Validation échouée
- `OrderExecuted` : Ordre exécuté au marché
- `OrderPartiallyFilled` : Exécution partielle
- `OrderCompleted` : Saga terminée
- `OrderCancelled` : Ordre annulé

### Implémentation dans BrokerX

Le système stocke tous les événements dans PostgreSQL avec garantie d'immutabilité (règles SQL bloquant UPDATE/DELETE).

**Requêtes d'audit** :

- Historique complet d'un ordre via `correlation_id`
- Reconstitution d'état à une date donnée (time travel)
- Traçabilité complète des sagas

## Avantages

**Audit immutable** : Historique complet impossible à modifier  
 **Time travel** : Reconstitution de l'état à n'importe quel moment  
 **Debugging** : Traçabilité complète des bugs  
 **Compliance** : Satisfaction des exigences réglementaires

## Inconvénients

**Stockage** : Volume de données plus important

## Références

- ADR 010 : Architecture événementielle
- ADR 011 : Transactional Outbox Pattern
