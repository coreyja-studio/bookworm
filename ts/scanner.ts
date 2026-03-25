// Barcode scanner for ISBN lookup — html5-qrcode bundled via esbuild.

import { Html5Qrcode, Html5QrcodeSupportedFormats } from "html5-qrcode";

interface IsbnResult {
  title: string;
  author: string;
  cover_url: string | null;
  isbn: string;
}

function initScanner(): void {
  const scanBtn = document.getElementById("scan-btn");
  const modal = document.getElementById("scanner-modal");
  const closeBtn = document.getElementById("scan-close");
  const status = document.getElementById("scan-status");

  if (!scanBtn || !modal || !closeBtn || !status) return;

  // Detect which page we're on: detail page has isbn-form, log page has title input
  const isbnForm = document.getElementById("isbn-form") as HTMLFormElement | null;
  const isDetailPage = !!isbnForm;

  let scanner: Html5Qrcode | null = null;

  function startScanner(): void {
    scanner = new Html5Qrcode("scanner-container", {
      formatsToSupport: [Html5QrcodeSupportedFormats.EAN_13, Html5QrcodeSupportedFormats.EAN_8],
      verbose: false,
      // Use native BarcodeDetector API on iOS/Chrome when available — much faster than WASM fallback
      useBarCodeDetectorIfSupported: true,
    });
    scanner
      .start(
        { facingMode: "environment" },
        {
          fps: 15,
          qrbox: { width: 280, height: 160 },
          disableFlip: true,
          videoConstraints: {
            facingMode: { ideal: "environment" },
            width: { ideal: 1920 },
            height: { ideal: 1080 },
            // Hint iOS to keep autofocus active (helps with macro lens switching)
            // @ts-expect-error: focusMode is a valid constraint on iOS Safari but not in TS's MediaTrackConstraints
            focusMode: { ideal: "continuous" },
          },
        },
        onScanSuccess,
        () => {},
      )
      .catch(() => {
        status!.textContent = "Camera access denied or not available";
      });
  }

  function onScanSuccess(decodedText: string): void {
    if (scanner) {
      scanner.stop().then(() => {
        scanner = null;
      });
    }
    status!.textContent = "Looking up ISBN: " + decodedText + "...";

    fetch("/api/isbn/" + encodeURIComponent(decodedText))
      .then((resp) => {
        if (!resp.ok) throw new Error("Lookup failed");
        return resp.json() as Promise<IsbnResult>;
      })
      .then((data) => {
        const isbnEl = document.getElementById("isbn") as HTMLInputElement | null;
        const coverEl = document.getElementById("cover_url") as HTMLInputElement | null;

        if (isbnEl) isbnEl.value = data.isbn;
        if (coverEl && data.cover_url) coverEl.value = data.cover_url;

        if (isDetailPage) {
          // On detail page: submit the form to save ISBN + cover
          modal!.classList.add("hidden");
          if (isbnForm) isbnForm.submit();
        } else {
          // On log page: also fill title and author
          const titleEl = document.getElementById("title") as HTMLInputElement | null;
          const authorEl = document.getElementById("author") as HTMLInputElement | null;

          if (titleEl) titleEl.value = data.title;
          if (authorEl) authorEl.value = data.author;

          modal!.classList.add("hidden");
        }
      })
      .catch(() => {
        status!.textContent = "Could not find book for ISBN: " + decodedText;
        setTimeout(() => {
          status!.textContent = "Point your camera at the book's barcode";
          startScanner();
        }, 2000);
      });
  }

  function closeModal(): void {
    if (scanner) {
      scanner
        .stop()
        .then(() => {
          scanner = null;
        })
        .catch(() => {});
    }
    modal!.classList.add("hidden");
  }

  scanBtn.addEventListener("click", () => {
    modal!.classList.remove("hidden");
    startScanner();
  });

  closeBtn.addEventListener("click", closeModal);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
  });
}

document.addEventListener("DOMContentLoaded", initScanner);
