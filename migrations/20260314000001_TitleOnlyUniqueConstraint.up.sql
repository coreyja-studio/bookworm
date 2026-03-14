-- Drop the (title, author) unique constraint and add a case-insensitive title-only unique index.
-- This prevents duplicate books with the same title regardless of author.
ALTER TABLE books DROP CONSTRAINT IF EXISTS books_title_author_key;

-- Auto-merge any existing duplicate titles before creating the unique index.
-- For each set of dupes, keep the earliest-created book and move reads from the rest.
WITH dupes AS (
    SELECT book_id, LOWER(title) as lower_title,
           ROW_NUMBER() OVER (PARTITION BY LOWER(title) ORDER BY created_at ASC) as rn
    FROM books
),
keepers AS (
    SELECT book_id, lower_title FROM dupes WHERE rn = 1
),
losers AS (
    SELECT d.book_id as loser_id, k.book_id as keeper_id
    FROM dupes d
    JOIN keepers k ON k.lower_title = d.lower_title
    WHERE d.rn > 1
)
UPDATE reads SET book_id = losers.keeper_id
FROM losers
WHERE reads.book_id = losers.loser_id;

-- Delete the now-orphaned duplicate book records
DELETE FROM books
WHERE book_id IN (
    SELECT book_id FROM (
        SELECT book_id,
               ROW_NUMBER() OVER (PARTITION BY LOWER(title) ORDER BY created_at ASC) as rn
        FROM books
    ) sub
    WHERE rn > 1
);

CREATE UNIQUE INDEX idx_books_title_unique ON books (LOWER(title));
