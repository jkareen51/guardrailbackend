ALTER TABLE treasury_status_snapshots
ADD COLUMN IF NOT EXISTS total_reserved_redemptions TEXT NOT NULL DEFAULT '0';

ALTER TABLE treasury_asset_snapshots
ADD COLUMN IF NOT EXISTS reserved_redemptions TEXT NOT NULL DEFAULT '0';

ALTER TABLE treasury_asset_snapshots
ADD COLUMN IF NOT EXISTS registered_asset_token BOOLEAN NOT NULL DEFAULT FALSE;
