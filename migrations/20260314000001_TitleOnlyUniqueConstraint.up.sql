-- Drop the (title, author) unique constraint and add a case-insensitive title-only unique index.
-- This prevents duplicate books with the same title regardless of author.
ALTER TABLE books DROP CONSTRAINT IF EXISTS books_title_author_key;

CREATE UNIQUE INDEX idx_books_title_unique ON books (LOWER(title));
