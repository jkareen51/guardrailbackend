ALTER TABLE asset_catalog_entries
DROP CONSTRAINT IF EXISTS asset_catalog_entries_asset_address_fkey;

ALTER TABLE asset_catalog_entries
ADD CONSTRAINT asset_catalog_entries_asset_address_fkey
FOREIGN KEY (asset_address) REFERENCES assets(asset_address)
ON DELETE CASCADE
ON UPDATE CASCADE;

ALTER TABLE asset_price_history
DROP CONSTRAINT IF EXISTS asset_price_history_asset_address_fkey;

ALTER TABLE asset_price_history
ADD CONSTRAINT asset_price_history_asset_address_fkey
FOREIGN KEY (asset_address) REFERENCES assets(asset_address)
ON DELETE CASCADE
ON UPDATE CASCADE;

ALTER TABLE oracle_valuation_history
DROP CONSTRAINT IF EXISTS oracle_valuation_history_asset_address_fkey;

ALTER TABLE oracle_valuation_history
ADD CONSTRAINT oracle_valuation_history_asset_address_fkey
FOREIGN KEY (asset_address) REFERENCES assets(asset_address)
ON DELETE CASCADE
ON UPDATE CASCADE;

ALTER TABLE user_trade_history
DROP CONSTRAINT IF EXISTS user_trade_history_asset_address_fkey;

ALTER TABLE user_trade_history
ADD CONSTRAINT user_trade_history_asset_address_fkey
FOREIGN KEY (asset_address) REFERENCES assets(asset_address)
ON DELETE CASCADE
ON UPDATE CASCADE;
