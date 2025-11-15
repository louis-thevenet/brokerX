# 📊 Event Store Schema - Order Processing Saga

## Vue d'ensemble

Ce schéma de base de données implémente une **architecture événementielle** avec **saga chorégraphiée** pour le passage d'ordres (UC-07). Il est **spécialisé** pour gérer uniquement le flux d'ordres, pas un event store générique.

## 🎯 Objectif

Implémenter le pattern **Saga Choreography** pour orchestrer le processus de passage d'ordres avec :
- **Fiabilité** : Transactional Outbox Pattern
- **Traçabilité** : Event Sourcing pour les ordres
- **Résilience** : État de saga pour compensation
- **Performance** : Consommation événementielle avec checkpoints

---

## 📋 Tables

### 1. `order_events` - Journal des Événements d'Ordres

**Rôle** : Stocke tous les événements du cycle de vie d'un ordre (Event Sourcing)

```sql
CREATE TABLE order_events (
    event_id UUID PRIMARY KEY,
    order_id UUID NOT NULL,
    user_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    
    -- Détails de l'ordre
    symbol VARCHAR(10) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL,    -- 'buy' | 'sell'
    order_type VARCHAR(20) NOT NULL,    -- 'market' | 'limit'
    price DECIMAL(15, 2),
    
    -- Résultats d'exécution
    filled_quantity BIGINT DEFAULT 0,
    average_price DECIMAL(15, 2),
    
    -- Métadonnées
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_number BIGSERIAL,
    correlation_id UUID,
    causation_id UUID,
    metadata JSONB
);
```

**Types d'événements** :
```
OrderCreated           → Ordre créé par l'API Gateway
OrderValidated         → Validation pré-trade réussie
OrderRejected          → Validation échouée (solde, holdings, risque)
OrderExecuted          → Envoyé au marché
OrderFilled            → Complètement exécuté
OrderPartiallyFilled   → Partiellement exécuté
FundsDebited           → Fonds débités (achat)
FundsCredited          → Fonds crédités (vente)
HoldingsUpdated        → Holdings mis à jour
OrderCompleted         → Saga complétée avec succès
```

**Exemple de flux** :
```
order-123: OrderCreated (t=0)
         ↓
order-123: OrderValidated (t=1)
         ↓
order-123: OrderExecuted (t=2)
         ↓
order-123: OrderFilled (quantity=100, price=150.50) (t=3)
         ↓
order-123: FundsDebited (amount=15050.00) (t=4)
         ↓
order-123: HoldingsUpdated (AAPL +100) (t=5)
         ↓
order-123: OrderCompleted (t=6)
```

---

### 2. `outbox` - Pattern Transactional Outbox

**Rôle** : Garantit la publication fiable des événements vers Redis/Kafka

```sql
CREATE TABLE outbox (
    outbox_id UUID PRIMARY KEY,
    order_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    processing_attempts INTEGER DEFAULT 0,
    last_error TEXT
);
```

**Workflow** :
1. **Transaction atomique** :
   ```sql
   BEGIN;
     INSERT INTO order_events (...);
     INSERT INTO outbox (...);
   COMMIT;
   ```

2. **Worker séparé** (outbox poller) :
   ```sql
   SELECT * FROM outbox 
   WHERE processed_at IS NULL 
   ORDER BY created_at 
   LIMIT 100;
   
   -- Pour chaque message
   Publish to Redis/Kafka
   UPDATE outbox SET processed_at = NOW()
   ```

3. **Retry avec backoff exponentiel** :
   - Tentative 1 : immédiat
   - Tentative 2 : +1s
   - Tentative 3 : +2s
   - Tentative 4 : +4s
   - Si échec après 10 tentatives → alerte monitoring

---

### 3. `saga_state` - État de la Saga

**Rôle** : Track l'état actuel de chaque saga d'ordre pour compensation

```sql
CREATE TABLE saga_state (
    order_id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    current_step VARCHAR(50) NOT NULL,
    
    -- Détails de l'ordre
    symbol VARCHAR(10) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL,
    order_type VARCHAR(20) NOT NULL,
    price DECIMAL(15, 2),
    
    -- Tracking
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    -- Résultats
    filled_quantity BIGINT DEFAULT 0,
    average_price DECIMAL(15, 2),
    
    -- Compensation
    error_message TEXT,
    compensation_needed BOOLEAN DEFAULT FALSE,
    
    correlation_id UUID NOT NULL,
    metadata JSONB
);
```

**Étapes de la saga** :
```
CREATED            → Ordre créé
  ↓
VALIDATING         → En cours de validation
  ↓
VALIDATED          → Validation réussie
  ↓
EXECUTING          → En cours d'exécution au marché
  ↓
EXECUTED           → Exécution terminée
  ↓
SETTLING           → En cours de settlement (fonds + holdings)
  ↓
COMPLETED          → Saga terminée avec succès

--- Chemins d'erreur ---
FAILED             → Échec (validation ou exécution)
COMPENSATING       → En cours de compensation
COMPENSATED        → Compensation terminée
```

**Compensation** :
Si une étape échoue après que des fonds ont été débités :
```sql
UPDATE saga_state 
SET current_step = 'COMPENSATING', 
    compensation_needed = TRUE
WHERE order_id = 'xxx';

-- Transactions compensatoires :
1. Rembourser les fonds débités
2. Annuler les holdings modifiés
3. Marquer la saga comme COMPENSATED
```

---

### 4. `subscriptions` - Checkpoints de Consommation

**Rôle** : Track la position de chaque consommateur dans le stream d'événements

```sql
CREATE TABLE subscriptions (
    subscription_id VARCHAR(100) PRIMARY KEY,
    last_processed_sequence BIGINT NOT NULL DEFAULT 0,
    last_processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Utilisation** :
```sql
-- Consumer "analytics-service" démarre
INSERT INTO subscriptions (subscription_id, last_processed_sequence)
VALUES ('analytics-service', 0);

-- Lire les nouveaux événements
SELECT * FROM order_events
WHERE sequence_number > (
    SELECT last_processed_sequence 
    FROM subscriptions 
    WHERE subscription_id = 'analytics-service'
)
ORDER BY sequence_number
LIMIT 1000;

-- Après traitement, mettre à jour le checkpoint
UPDATE subscriptions
SET last_processed_sequence = 1234,
    last_processed_at = NOW()
WHERE subscription_id = 'analytics-service';
```

---

## 🔄 Workflow Complet : Achat de 100 AAPL @ $150

### Étape 1 : Création de l'ordre (API Gateway)

```sql
BEGIN;

-- Event: OrderCreated
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity, 
    order_side, order_type, price, correlation_id
) VALUES (
    'order-123', 'user-456', 'OrderCreated', 'AAPL', 100,
    'buy', 'limit', 150.00, 'corr-789'
);

-- Outbox: Publier l'événement
INSERT INTO outbox (order_id, event_type, payload)
VALUES (
    'order-123', 'OrderCreated',
    '{"order_id": "order-123", "user_id": "user-456", ...}'
);

-- Saga: Initialiser l'état
INSERT INTO saga_state (
    order_id, user_id, current_step, symbol, quantity,
    order_side, order_type, price, correlation_id
) VALUES (
    'order-123', 'user-456', 'CREATED', 'AAPL', 100,
    'buy', 'limit', 150.00, 'corr-789'
);

COMMIT;
```

### Étape 2 : Validation (Order Execution Orchestrator)

```sql
BEGIN;

-- Event: OrderValidated
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity,
    order_side, order_type, correlation_id, causation_id,
    metadata
) VALUES (
    'order-123', 'user-456', 'OrderValidated', 'AAPL', 100,
    'buy', 'limit', 'corr-789', 'evt-created-123',
    '{"balance_ok": true, "holdings_ok": true, "risk_ok": true}'
);

-- Outbox
INSERT INTO outbox (order_id, event_type, payload)
VALUES ('order-123', 'OrderValidated', '...');

-- Saga: Mettre à jour
UPDATE saga_state
SET current_step = 'VALIDATED'
WHERE order_id = 'order-123';

COMMIT;
```

### Étape 3 : Exécution (Market Data Service)

```sql
BEGIN;

-- Event: OrderFilled
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity,
    order_side, order_type, correlation_id,
    filled_quantity, average_price,
    metadata
) VALUES (
    'order-123', 'user-456', 'OrderFilled', 'AAPL', 100,
    'buy', 'limit', 'corr-789',
    100, 150.50,
    '{"market_price": 150.45, "slippage": 0.05}'
);

-- Outbox
INSERT INTO outbox (order_id, event_type, payload)
VALUES ('order-123', 'OrderFilled', '...');

-- Saga
UPDATE saga_state
SET current_step = 'EXECUTED',
    filled_quantity = 100,
    average_price = 150.50
WHERE order_id = 'order-123';

COMMIT;
```

### Étape 4 : Settlement - Débiter les fonds

```sql
BEGIN;

-- Event: FundsDebited
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity,
    order_side, order_type, correlation_id,
    filled_quantity, average_price,
    metadata
) VALUES (
    'order-123', 'user-456', 'FundsDebited', 'AAPL', 100,
    'buy', 'limit', 'corr-789',
    100, 150.50,
    '{"amount": 15050.00, "new_balance": 84950.00}'
);

-- Outbox
INSERT INTO outbox (order_id, event_type, payload)
VALUES ('order-123', 'FundsDebited', '...');

-- Saga
UPDATE saga_state
SET current_step = 'SETTLING'
WHERE order_id = 'order-123';

COMMIT;
```

### Étape 5 : Settlement - Mettre à jour les holdings

```sql
BEGIN;

-- Event: HoldingsUpdated
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity,
    order_side, order_type, correlation_id,
    filled_quantity, average_price,
    metadata
) VALUES (
    'order-123', 'user-456', 'HoldingsUpdated', 'AAPL', 100,
    'buy', 'limit', 'corr-789',
    100, 150.50,
    '{"symbol": "AAPL", "delta": 100, "new_quantity": 100}'
);

-- Outbox
INSERT INTO outbox (order_id, event_type, payload)
VALUES ('order-123', 'HoldingsUpdated', '...');

COMMIT;
```

### Étape 6 : Complétion

```sql
BEGIN;

-- Event: OrderCompleted
INSERT INTO order_events (
    order_id, user_id, event_type, symbol, quantity,
    order_side, order_type, correlation_id,
    filled_quantity, average_price
) VALUES (
    'order-123', 'user-456', 'OrderCompleted', 'AAPL', 100,
    'buy', 'limit', 'corr-789',
    100, 150.50
);

-- Outbox
INSERT INTO outbox (order_id, event_type, payload)
VALUES ('order-123', 'OrderCompleted', '...');

-- Saga: Terminer
UPDATE saga_state
SET current_step = 'COMPLETED',
    completed_at = NOW()
WHERE order_id = 'order-123';

COMMIT;
```

---

## 🎯 Avantages de ce Schéma

| Fonctionnalité | Implémentation | Bénéfice |
|----------------|----------------|----------|
| **Traçabilité complète** | `order_events` | Audit complet, debugging facilité |
| **Publication fiable** | `outbox` | Exactly-once delivery garantie |
| **Compensation** | `saga_state` | Gestion d'erreurs et rollback |
| **Performance** | `subscriptions` | At-least-once processing efficace |
| **Corrélation** | `correlation_id` | Traçage distribué des sagas |

---

## 🔍 Requêtes Utiles

### Voir tous les événements d'un ordre
```sql
SELECT event_type, occurred_at, filled_quantity, average_price, metadata
FROM order_events
WHERE order_id = 'order-123'
ORDER BY sequence_number;
```

### Ordres en échec nécessitant compensation
```sql
SELECT order_id, current_step, error_message, started_at
FROM saga_state
WHERE compensation_needed = TRUE 
  AND completed_at IS NULL
ORDER BY started_at;
```

### Messages outbox non traités
```sql
SELECT order_id, event_type, processing_attempts, created_at
FROM outbox
WHERE processed_at IS NULL
ORDER BY created_at;
```

### Activité récente par utilisateur
```sql
SELECT event_type, symbol, quantity, order_side, occurred_at
FROM order_events
WHERE user_id = 'user-456'
ORDER BY occurred_at DESC
LIMIT 50;
```

