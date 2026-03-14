ALTER TABLE books ADD COLUMN isbn TEXT;
ALTER TABLE books ADD COLUMN cover_url TEXT;

CREATE UNIQUE INDEX idx_books_isbn ON books(isbn) WHERE isbn IS NOT NULL;
