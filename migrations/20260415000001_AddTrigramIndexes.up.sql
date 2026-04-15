-- Enable pg_trgm extension for fuzzy text matching (trigram similarity)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- GIN trigram indexes for fast fuzzy search on title and author
CREATE INDEX IF NOT EXISTS idx_books_title_trgm ON books USING GIN (title gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_books_author_trgm ON books USING GIN (author gin_trgm_ops);
