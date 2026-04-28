SELECT
    asset_address,
    document_type,
    document_hash,
    reference_id,
    updated_by_user_id,
    last_tx_hash,
    created_at,
    updated_at
FROM oracle_documents
WHERE asset_address = $1
  AND document_type = $2
