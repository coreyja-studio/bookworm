DROP INDEX idx_reads_not_deleted;
ALTER TABLE reads DROP COLUMN deleted_at;
