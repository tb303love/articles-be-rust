-- Početna šema baze, rekonstruisana iz `artikli_dev` (stanje posle migracije 20261027120000).
-- Zamenjuje izgubljene migracije 20260714202727, 20260714212604 i 20260714213300 koje nikad nisu ušle u git.
--
-- `IF NOT EXISTS`: na postojećim bazama je ovo no-op (samo se upiše u _sqlx_migrations),
-- a na čistoj instalaciji pravi kompletnu šemu. Tabele su poređane po zavisnostima (FK).

CREATE TABLE IF NOT EXISTS `users` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `username` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `password` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `role` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_users_username` (`username`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `refresh_tokens` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `token` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `expiry_date` datetime(6) NOT NULL,
  `user_id` bigint unsigned DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_refresh_tokens_token` (`token`),
  UNIQUE KEY `uk_refresh_tokens_user_id` (`user_id`),
  CONSTRAINT `fk_refresh_tokens_users` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- --- KATEGORIJE ---

CREATE TABLE IF NOT EXISTS `category_name` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_category_name_name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `category` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `category_name_id` bigint unsigned DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_category_category_name` (`category_name_id`),
  CONSTRAINT `fk_category_category_name` FOREIGN KEY (`category_name_id`) REFERENCES `category_name` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- --- ARTIKLI ---

CREATE TABLE IF NOT EXISTS `article_images` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `file_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `file_type` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `original_name` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `article` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `price` decimal(19,4) DEFAULT NULL,
  `admission_price1` decimal(19,4) DEFAULT NULL,
  `admission_price2` decimal(19,4) DEFAULT NULL,
  `category_id` bigint unsigned DEFAULT NULL,
  `article_image_id` bigint unsigned DEFAULT NULL,
  `active` bit(1) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_article_name` (`name`),
  UNIQUE KEY `uk_article_image_id` (`article_image_id`),
  KEY `fk_article_category` (`category_id`),
  CONSTRAINT `fk_article_category` FOREIGN KEY (`category_id`) REFERENCES `category` (`id`) ON DELETE SET NULL,
  CONSTRAINT `fk_article_image` FOREIGN KEY (`article_image_id`) REFERENCES `article_images` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `article_barcodes` (
  `article_id` bigint unsigned NOT NULL,
  `barcode` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  UNIQUE KEY `uk_article_barcodes_barcode` (`barcode`),
  KEY `fk_article_barcodes_article` (`article_id`),
  CONSTRAINT `fk_article_barcodes_article` FOREIGN KEY (`article_id`) REFERENCES `article` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `article_composition` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `quantity` int unsigned NOT NULL,
  `bundle_id` bigint unsigned NOT NULL,
  `component_id` bigint unsigned NOT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_composition_bundle` (`bundle_id`),
  KEY `fk_composition_component` (`component_id`),
  CONSTRAINT `fk_composition_bundle` FOREIGN KEY (`bundle_id`) REFERENCES `article` (`id`) ON DELETE CASCADE,
  CONSTRAINT `fk_composition_component` FOREIGN KEY (`component_id`) REFERENCES `article` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `article_stock` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `article_id` bigint unsigned NOT NULL,
  `batch_number` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `expiration_date` date DEFAULT NULL,
  `quantity` bigint unsigned NOT NULL,
  `received_at` datetime(6) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_article_stock_article` (`article_id`),
  CONSTRAINT `fk_article_stock_article` FOREIGN KEY (`article_id`) REFERENCES `article` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- --- PRODAJA I REFUNDACIJE ---

CREATE TABLE IF NOT EXISTS `orders` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `created_at` datetime(6) DEFAULT NULL,
  `total_amount` decimal(19,4) DEFAULT NULL,
  `type` enum('RETAIL','WHOLESALE') COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `is_gratis` bit(1) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `order_articles` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `order_id` bigint unsigned NOT NULL,
  `article_id` bigint unsigned DEFAULT NULL,
  `article_name_snapshot` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `price_at_sale` decimal(19,4) NOT NULL,
  `admission_price_at_sale` decimal(19,4) NOT NULL,
  `admission_price2_at_sale` decimal(19,4) DEFAULT NULL,
  `quantity` int NOT NULL,
  `refunded_quantity` int NOT NULL,
  `batch_number` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `expiration_date` date DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_order_articles_orders` (`order_id`),
  KEY `fk_order_articles_article` (`article_id`),
  CONSTRAINT `fk_order_articles_article` FOREIGN KEY (`article_id`) REFERENCES `article` (`id`) ON DELETE SET NULL,
  CONSTRAINT `fk_order_articles_orders` FOREIGN KEY (`order_id`) REFERENCES `orders` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `refunds` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `created_at` datetime(6) DEFAULT NULL,
  `total_amount` decimal(19,4) DEFAULT NULL,
  `original_order_id` bigint unsigned DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_refunds_orders` (`original_order_id`),
  CONSTRAINT `fk_refunds_orders` FOREIGN KEY (`original_order_id`) REFERENCES `orders` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `refund_items` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `refund_id` bigint unsigned DEFAULT NULL,
  `article_id` bigint unsigned DEFAULT NULL,
  `price_at_refund` decimal(19,4) DEFAULT NULL,
  `quantity` int DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_refund_items_refunds` (`refund_id`),
  KEY `fk_refund_items_article` (`article_id`),
  CONSTRAINT `fk_refund_items_article` FOREIGN KEY (`article_id`) REFERENCES `article` (`id`) ON DELETE SET NULL,
  CONSTRAINT `fk_refund_items_refunds` FOREIGN KEY (`refund_id`) REFERENCES `refunds` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- --- OTPIS ZALIHA ---

CREATE TABLE IF NOT EXISTS `write_offs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `created_at` datetime(6) NOT NULL,
  `napomena` text COLLATE utf8mb4_unicode_ci,
  `ukupna_vrednost` decimal(19,4) NOT NULL,
  `created_by` bigint unsigned DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_write_offs_users` (`created_by`),
  CONSTRAINT `fk_write_offs_users` FOREIGN KEY (`created_by`) REFERENCES `users` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS `write_off_items` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `write_off_id` bigint unsigned NOT NULL,
  `article_stock_id` bigint unsigned NOT NULL,
  `quantity` int NOT NULL,
  `price_at_write_off` decimal(19,4) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_wo_items_write_offs` (`write_off_id`),
  KEY `fk_wo_items_stock` (`article_stock_id`),
  CONSTRAINT `fk_wo_items_stock` FOREIGN KEY (`article_stock_id`) REFERENCES `article_stock` (`id`) ON DELETE CASCADE,
  CONSTRAINT `fk_wo_items_write_offs` FOREIGN KEY (`write_off_id`) REFERENCES `write_offs` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- --- IZVEŠTAJI ---

CREATE TABLE IF NOT EXISTS `email_report_logs` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `message` text COLLATE utf8mb4_unicode_ci,
  `status` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `timestamp` datetime(6) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
