-- Add migration script here
DROP TABLE IF EXISTS Sessions;

DROP FUNCTION IF EXISTS update_updated_at_column ();
