DROP INDEX IF EXISTS idx_books_title_unique;

ALTER TABLE books ADD CONSTRAINT books_title_author_key UNIQUE (title, author);
