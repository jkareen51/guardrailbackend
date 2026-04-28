INSERT INTO oracle_documents (
    asset_address,
    document_type,
    document_hash,
    reference_id,
    updated_by_user_id,
    last_tx_hash
) VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (asset_address, document_type) DO UPDATE
SET
    document_hash = EXCLUDED.document_hash,
    reference_id = EXCLUDED.reference_id,
    updated_by_user_id = EXCLUDED.updated_by_user_id,
    last_tx_hash = COALESCE(EXCLUDED.last_tx_hash, oracle_documents.last_tx_hash),
    updated_at = NOW()
RETURNING
    asset_address,
    document_type,
    document_hash,
    reference_id,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
