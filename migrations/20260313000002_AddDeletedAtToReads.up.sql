ALTER TABLE reads ADD COLUMN deleted_at TIMESTAMPTZ;
CREATE INDEX idx_reads_not_deleted ON reads(book_id, read_date) WHERE deleted_at IS NULL;
