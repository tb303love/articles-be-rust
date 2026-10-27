use crate::dto::{ArticleCategory, ArticleCategoryName, CategoryNameResponse};
use sqlx::{MySqlConnection, MySqlPool, Row};

pub struct SqlxCategoryRepository {
    pool: MySqlPool,
}

impl SqlxCategoryRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn get_all_category_names(
        &self,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<CategoryNameResponse>, sqlx::Error> {
        // Čitamo direktno iz šifarnika category_name
        let rows = sqlx::query("SELECT id, name FROM category_name")
            .fetch_all(conn)
            .await?;

        let categories = rows
            .into_iter()
            .map(|row| {
                let id = row.get("id"); // MySQL BIGINT/INT se u sqlx-u često čita kao i64
                let name: String = row.get("name");

                CategoryNameResponse { id, name }
            })
            .collect();

        Ok(categories)
    }

    /// Traži unikatno ime kategorije u tabeli `category_name` na osnovu teksta (case-insensitive).
    pub async fn find_name_by_text(
        &self,
        conn: &mut MySqlConnection,
        name: &str,
    ) -> Result<Option<u64>, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT id FROM category_name WHERE LOWER(name) = LOWER(?)",
            name.trim()
        )
        .fetch_optional(conn) // 👈 Prosleđujemo conn umesto &self.pool
        .await?;

        Ok(result.map(|row| row.id))
    }

    pub async fn create_category_name(
        &self,
        conn: &mut MySqlConnection,
        name: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("INSERT INTO category_name (name) VALUES (?)", name.trim())
            .execute(conn) // 👈 Prosleđujemo conn
            .await?;

        Ok(result.last_insert_id())
    }

    /// Pravi novi red u tabeli category unutar transakcije
    pub async fn create_category(
        &self,
        conn: &mut MySqlConnection,
        category_name_id: u64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "INSERT INTO category (category_name_id) VALUES (?)",
            category_name_id
        )
        .execute(conn) // 👈 Prosleđujemo conn
        .await?;

        Ok(result.last_insert_id())
    }

    /// Proverava da li u tabeli `category` već postoji red koji pokazuje na dati `category_name_id`.
    pub async fn find_category_by_name_id(
        &self,
        conn: &mut MySqlConnection,
        name_id: u64,
    ) -> Result<Option<u64>, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT id FROM category WHERE category_name_id = ?",
            name_id
        )
        .fetch_optional(conn) // 👈 Prosleđujemo conn
        .await?;

        Ok(result.map(|row| row.id))
    }

    pub async fn find_categories_by_ids(
        &self,
        category_ids: &[u64],
    ) -> Result<Vec<ArticleCategory>, sqlx::Error> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Inicijalizujemo QueryBuilder sa statičkim početkom upita
        // 1. Inicijalizacija bez komplikovanih generičkih tipova sa strane
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT c.id AS category_id, cn.id AS name_id, cn.name AS category_name \
     FROM category c \
     INNER JOIN category_name cn ON c.category_name_id = cn.id \
     WHERE c.id IN (",
        );

        // 2. Bezbedno generišemo placeholder-e i automatski vezujemo (bind) vrednosti odjednom
        let mut separated = query_builder.separated(", ");
        for id in category_ids {
            separated.push_bind(*id as i64); // push_bind stavlja "?" i odmah radi .bind() iza kulisa
        }
        separated.push_unseparated(")"); // Zatvaramo IN zagradu

        // 3. Izvršavamo sklopljeni upit
        let query = query_builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        // 4. Mapiranje rezultata (ostaje isto)
        let categories = rows
            .into_iter()
            .map(|row| {
                let c_id = row.get("category_id");
                let n_id = row.get("name_id");
                let name: String = row.get("category_name");

                ArticleCategory {
                    id: c_id,
                    category_name: ArticleCategoryName { id: n_id, name },
                }
            })
            .collect();

        Ok(categories)
    }

    /// Briše sve kategorije i imena iz baze koji više nemaju nijedan povezan artikal.
    pub async fn clean_unused_categories(
        &self,
        conn: &mut sqlx::MySqlConnection,
    ) -> Result<(), sqlx::Error> {
        // 1. Brišemo redove iz `category` tabele koji se više ne nalaze ni u jednom artiklu
        sqlx::query!(
        "DELETE FROM category WHERE id NOT IN (SELECT DISTINCT category_id FROM article WHERE category_id IS NOT NULL)"
    )
         .execute(&mut *conn)
         .await?;

        // 2. Brišemo redove iz `category_name` koji više nemaju vezu u `category` tabeli
        sqlx::query!(
        // `IS NOT NULL` je obavezan: jedan NULL u podupitu bi učinio da NOT IN ne obriše ništa
        "DELETE FROM category_name WHERE id NOT IN (SELECT DISTINCT category_name_id FROM category WHERE category_name_id IS NOT NULL)"
    )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn resolve_from_input(
        &self,
        category_name_input: &Option<String>,
        conn: &mut sqlx::MySqlConnection,
    ) -> Result<Option<u64>, sqlx::Error> {
        let trimmed_name = category_name_input.as_ref().map(|s| s.trim().to_string());

        if trimmed_name.is_none() || trimmed_name.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        let name = trimmed_name.unwrap();

        if let Some(name_id) = self.find_name_by_text(&mut *conn, &name).await? {
            if let Some(category_id) = self.find_category_by_name_id(&mut *conn, name_id).await? {
                return Ok(Some(category_id));
            }
            let category_id = self.create_category(&mut *conn, name_id).await?;
            return Ok(Some(category_id));
        }

        let new_name_id = self.create_category_name(&mut *conn, &name).await?;
        let new_category_id = self.create_category(&mut *conn, new_name_id).await?;

        Ok(Some(new_category_id))
    }
}
