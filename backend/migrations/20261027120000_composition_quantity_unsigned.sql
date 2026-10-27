-- Rust model `ArticleComposition.quantity` je u64, a sqlx odbija da dekodira signed INT u u64.
-- Bez ove izmene svaki upit nad sastavnicama (spisak artikala sa paketom) vraća grešku.

-- Stari redovi (Java/Hibernate) mogu imati NULL ili nevalidnu količinu, što bi oborilo ALTER ispod.
-- Komponenta bez količine nema smisla u paketu, pa je takve redove najbezbednije postaviti na 1.
UPDATE article_composition SET quantity = 1 WHERE quantity IS NULL OR quantity < 1;

ALTER TABLE article_composition MODIFY quantity INT UNSIGNED NOT NULL;
