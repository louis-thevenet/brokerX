# ADR 007 – Transactional Outbox Pattern

## Statut

Acceptée

## Contexte

Dans une architecture événementielle, garantir que les événements sont publiés de manière fiable est critique. Le problème classique :

1. Service met à jour sa base de données
2. Service publie un événement vers Redis
3. **CRASH** entre les deux étapes

**Conséquence** : La base de données est mise à jour mais l'événement n'est jamais publié, créant une incohérence permanente.

## Décision

Implémentation du **Transactional Outbox Pattern** dans Event Store Service pour garantir l'atomicité entre mise à jour locale et publication d'événements.

## Implémentation

### Principe

1. **Transaction unique** : L'événement est sauvegardé dans `order_events` ET `outbox` dans la même transaction PostgreSQL
2. **Worker asynchrone** : Un worker poll la table `outbox` toutes les 100ms et publie vers Redis
3. **Idempotence** : Chaque événement a un `event_id` unique pour éviter les duplications

### Schema de la table Outbox

```sql
CREATE TABLE outbox (
    id BIGSERIAL PRIMARY KEY,
    order_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    processing_attempts INTEGER DEFAULT 0,
    last_error TEXT
);
```

### Flux de traitement

API Gateway -> Event Store Service -> Transaction BEGIN -> INSERT INTO order_events -> INSERT INTO outbox -> Transaction COMMIT -> [Outbox Worker] -> PUBLISH to Redis → Mark processed

## Conséquences

### Avantages

- **Fiabilité** : Publication garantie même en cas de crash
- **Cohérence** : Événements et base de données toujours synchronisés
- **Résilience** : Retry automatique en cas d'erreur réseau
- **Observabilité** : Monitoring des événements en attente

### Inconvénients

- **Latence** : Délai de polling (100ms)
- **Complexité** : Worker supplémentaire à maintenir
- **Stockage** : Événements dupliqués dans `order_events` et `outbox`

## Implémentation dans BrokerX

- **Localisation** : Event Store Service
- **Worker** : Tâche Tokio polling toutes les 100ms
- **Métriques** : Prometheus pour surveillance du backlog
- **Cleanup** : Événements publiés supprimés après 7 jours
