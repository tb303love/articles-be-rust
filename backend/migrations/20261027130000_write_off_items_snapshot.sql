-- Stavka otpisa više ne zavisi od reda u `article_stock`.
-- `update_article` pri svakoj izmeni briše i ponovo upisuje serije, a sa ON DELETE CASCADE
-- to je brisalo i stavke otpisa. Sada stavka čuva svoj snapshot (kao `order_articles`),
-- a veze ka seriji i artiklu se samo prekidaju (SET NULL) kada ti redovi nestanu.

ALTER TABLE write_off_items
    ADD COLUMN article_id bigint unsigned DEFAULT NULL AFTER article_stock_id,
    ADD COLUMN article_name_snapshot varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL DEFAULT '' AFTER article_id,
    ADD COLUMN batch_number_snapshot varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL AFTER article_name_snapshot;

-- Postojeće stavke dobijaju snapshot iz serije i artikla na koje trenutno pokazuju
UPDATE write_off_items i
    JOIN article_stock s ON s.id = i.article_stock_id
    JOIN article a ON a.id = s.article_id
SET i.article_id = a.id,
    i.article_name_snapshot = a.name,
    i.batch_number_snapshot = s.batch_number;

-- Default je služio samo za popunjavanje postojećih redova, nove stavke moraju same da pošalju ime
ALTER TABLE write_off_items ALTER COLUMN article_name_snapshot DROP DEFAULT;

ALTER TABLE write_off_items DROP FOREIGN KEY fk_wo_items_stock;

ALTER TABLE write_off_items
    MODIFY article_stock_id bigint unsigned DEFAULT NULL,
    ADD CONSTRAINT fk_wo_items_stock FOREIGN KEY (article_stock_id) REFERENCES article_stock (id) ON DELETE SET NULL,
    ADD CONSTRAINT fk_wo_items_article FOREIGN KEY (article_id) REFERENCES article (id) ON DELETE SET NULL;
