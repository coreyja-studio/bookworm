// Barcode scanner for ISBN lookup — loaded alongside html5-qrcode from CDN.
// Types come from the html5-qrcode devDependency; the library itself is a global at runtime.

import type {
  Html5Qrcode as Html5QrcodeType,
  Html5QrcodeSupportedFormats as Html5QrcodeSupportedFormatsType,
} from "html5-qrcode";

// Globals provided by the html5-qrcode CDN script
declare const Html5Qrcode: typeof Html5QrcodeType;
declare const Html5QrcodeSupportedFormats: typeof Html5QrcodeSupportedFormatsType;

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

  let scanner: Html5QrcodeType | null = null;

  function startScanner(): void {
    scanner = new Html5Qrcode("scanner-container", {
      formatsToSupport: [Html5QrcodeSupportedFormats.EAN_13, Html5QrcodeSupportedFormats.EAN_8],
      verbose: false,
    });
    scanner
      .start(
        { facingMode: "environment" },
        { fps: 10, qrbox: { width: 250, height: 150 } },
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
        const titleEl = document.getElementById("title") as HTMLInputElement | null;
        const authorEl = document.getElementById("author") as HTMLInputElement | null;
        const isbnEl = document.getElementById("isbn") as HTMLInputElement | null;
        const coverEl = document.getElementById("cover_url") as HTMLInputElement | null;

        if (titleEl) titleEl.value = data.title;
        if (authorEl) authorEl.value = data.author;
        if (isbnEl) isbnEl.value = data.isbn;
        if (coverEl && data.cover_url) coverEl.value = data.cover_url;

        modal!.classList.add("hidden");
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
    if (typeof Html5Qrcode === "undefined") {
      alert("Scanner not available. Please enter the book details manually.");
      return;
    }
    modal!.classList.remove("hidden");
    startScanner();
  });

  closeBtn.addEventListener("click", closeModal);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
  });
}

document.addEventListener("DOMContentLoaded", initScanner);
