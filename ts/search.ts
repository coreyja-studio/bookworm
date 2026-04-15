// Live fuzzy search for books — debounced fetch against /api/search

interface BookResult {
  book_id: string;
  title: string;
  author: string;
  read_count: number;
  has_cover: boolean;
}

function initSearch(): void {
  const input = document.getElementById("search-input") as HTMLInputElement | null;
  const results = document.getElementById("search-results");
  const formCard = document.getElementById("log-form-card");

  if (!input || !results) return;

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let abortController: AbortController | null = null;

  function renderResults(books: BookResult[]): void {
    if (!results) return;

    if (books.length === 0) {
      results.innerHTML = "";
      results.classList.add("hidden");
      // Show the manual form when no results
      if (formCard) formCard.classList.remove("hidden");
      return;
    }

    // Hide the manual form card while showing results
    if (formCard) formCard.classList.add("hidden");

    results.classList.remove("hidden");
    results.innerHTML = books
      .map((book) => {
        const authorLine = book.author
          ? `<div class="text-sm" style="color: var(--color-subtext)">by ${escapeHtml(book.author)}</div>`
          : "";
        const countBadge =
          book.read_count > 0
            ? `<span class="text-xs font-bold px-2 py-0.5 rounded-full" style="background: var(--color-accent-bg-purple); color: var(--color-accent-purple)">read ${book.read_count}x</span>`
            : `<span class="text-xs font-bold px-2 py-0.5 rounded-full" style="background: var(--color-accent-bg-green); color: var(--color-accent-green)">new</span>`;
        return `<button type="button" class="search-result w-full text-left p-3 rounded-xl hover:bg-orange-50 transition-colors flex items-center gap-3 haptic-light"
                  data-book-id="${escapeHtml(book.book_id)}"
                  data-title="${escapeHtml(book.title)}"
                  data-author="${escapeHtml(book.author)}">
            <div class="flex-1 min-w-0">
              <div class="font-bold truncate">${escapeHtml(book.title)}</div>
              ${authorLine}
            </div>
            ${countBadge}
          </button>`;
      })
      .join("");

    // Add "Log as new book" option at the bottom
    const query = input?.value.trim() ?? "";
    if (query) {
      results.innerHTML += `<button type="button" id="search-new-book" class="w-full text-left p-3 rounded-xl transition-colors flex items-center gap-3 haptic-light" style="border: 2px dashed var(--color-card-border)">
          <div class="flex-1 min-w-0">
            <div class="font-bold" style="color: var(--color-accent-orange)">+ Log "${escapeHtml(query)}" as a new book</div>
          </div>
        </button>`;

      const newBookBtn = document.getElementById("search-new-book");
      if (newBookBtn) {
        newBookBtn.addEventListener("click", () => {
          // Show the form, fill in the title
          if (formCard) formCard.classList.remove("hidden");
          results!.classList.add("hidden");
          const titleEl = document.getElementById("title") as HTMLInputElement | null;
          if (titleEl) {
            titleEl.value = query;
            titleEl.focus();
          }
        });
      }
    }

    // Wire up click handlers for each result
    results.querySelectorAll(".search-result").forEach((btn) => {
      btn.addEventListener("click", () => {
        const bookId = (btn as HTMLElement).dataset.bookId;
        if (!bookId) return;

        // Submit a re-read for this book
        const form = document.createElement("form");
        form.method = "POST";
        form.action = "/log/reread";
        const input = document.createElement("input");
        input.type = "hidden";
        input.name = "book_id";
        input.value = bookId;
        form.appendChild(input);
        document.body.appendChild(form);
        form.submit();
      });
    });
  }

  function doSearch(query: string): void {
    if (abortController) {
      abortController.abort();
    }

    if (query.length < 2) {
      renderResults([]);
      return;
    }

    abortController = new AbortController();

    fetch(`/api/search?q=${encodeURIComponent(query)}`, {
      signal: abortController.signal,
    })
      .then((resp) => {
        if (!resp.ok) throw new Error("Search failed");
        return resp.json() as Promise<BookResult[]>;
      })
      .then((books) => {
        renderResults(books);
      })
      .catch((err) => {
        if (err instanceof Error && err.name === "AbortError") return;
        console.error("Search error:", err);
      });
  }

  input.addEventListener("input", () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    const query = input.value.trim();
    debounceTimer = setTimeout(() => doSearch(query), 200);
  });

  // Show/hide form based on search state
  input.addEventListener("focus", () => {
    if (input.value.trim().length >= 2) {
      doSearch(input.value.trim());
    }
  });

  // Dismiss results when tapping outside the search area.
  // Track whether a touch moved (scroll) so we don't dismiss on scroll.
  let touchMoved = false;
  document.addEventListener("touchstart", () => {
    touchMoved = false;
  });
  document.addEventListener("touchmove", () => {
    touchMoved = true;
  });
  document.addEventListener("touchend", (e) => {
    if (touchMoved) return; // was a scroll, not a tap
    const target = e.target as HTMLElement;
    const searchArea = document.getElementById("search-area");
    if (searchArea && !searchArea.contains(target)) {
      dismissResults();
    }
  });

  // Desktop: dismiss on click outside
  document.addEventListener("mousedown", (e) => {
    const target = e.target as HTMLElement;
    const searchArea = document.getElementById("search-area");
    if (searchArea && !searchArea.contains(target)) {
      dismissResults();
    }
  });

  function dismissResults(): void {
    results!.classList.add("hidden");
    if (formCard) formCard.classList.remove("hidden");
  }
}

function escapeHtml(str: string): string {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

document.addEventListener("DOMContentLoaded", initSearch);
