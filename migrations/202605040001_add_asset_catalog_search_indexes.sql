CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX IF NOT EXISTS assets_name_trgm_idx
ON assets USING gin (name gin_trgm_ops);

CREATE INDEX IF NOT EXISTS assets_symbol_trgm_idx
ON assets USING gin (symbol gin_trgm_ops);

CREATE INDEX IF NOT EXISTS asset_catalog_entries_slug_trgm_idx
ON asset_catalog_entries USING gin (slug gin_trgm_ops);

CREATE INDEX IF NOT EXISTS asset_catalog_entries_summary_trgm_idx
ON asset_catalog_entries USING gin (COALESCE(summary, '') gin_trgm_ops);

CREATE INDEX IF NOT EXISTS asset_catalog_entries_market_segment_trgm_idx
ON asset_catalog_entries USING gin (COALESCE(market_segment, '') gin_trgm_ops);

CREATE INDEX IF NOT EXISTS asset_catalog_entries_suggested_internal_tags_gin_idx
ON asset_catalog_entries USING gin (suggested_internal_tags);
