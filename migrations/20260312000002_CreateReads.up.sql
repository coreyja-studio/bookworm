CREATE TABLE reads (
    read_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES books(book_id),
    read_date DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reads_book_id ON reads(book_id);
CREATE INDEX idx_reads_read_date ON reads(read_date);
