use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct User {
    name: String,
    email: String,
}

#[tokio::test]
async fn test_postgres_repo_crud() -> anyhow::Result<()> {
    use crate::db::{PostgresRepo, Repository};
    // Each test uses a fresh table to avoid conflicts
    let table = format!(
        "users_test_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );

    let repo = PostgresRepo::<User, String>::new(&table).await?;

    // Insert
    let user = User {
        name: "Alice".into(),
        email: "alice@example.com".into(),
    };
    repo.insert("1".to_string(), user.clone()).await?;

    // Get
    let fetched = repo.get(&"1".to_string()).await?;
    assert_eq!(fetched, Some(user.clone()));

    // Update
    let updated = User {
        name: "Alice Updated".into(),
        email: "alice@newmail.com".into(),
    };
    repo.update("1".to_string(), updated.clone()).await?;
    let fetched2 = repo.get(&"1".to_string()).await?;
    assert_eq!(fetched2, Some(updated.clone()));

    // Len
    let count = repo.len().await?;
    assert_eq!(count, 1);

    // Remove
    repo.remove("1".to_string()).await?;
    let fetched3 = repo.get(&"1".to_string()).await?;
    assert!(fetched3.is_none());

    Ok(())
}
