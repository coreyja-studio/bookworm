function initScanner() {
	const scanBtn = document.getElementById("scan-btn");
	const modal = document.getElementById("scanner-modal");
	const closeBtn = document.getElementById("scan-close");
	const status = document.getElementById("scan-status");
	if (!scanBtn || !modal || !closeBtn || !status) return;
	let scanner = null;
	function startScanner() {
		scanner = new Html5Qrcode("scanner-container", {
			formatsToSupport: [Html5QrcodeSupportedFormats.EAN_13, Html5QrcodeSupportedFormats.EAN_8],
			verbose: false
		});
		scanner.start({ facingMode: "environment" }, {
			fps: 10,
			qrbox: {
				width: 250,
				height: 150
			}
		}, onScanSuccess, () => {}).catch(() => {
			status.textContent = "Camera access denied or not available";
		});
	}
	function onScanSuccess(decodedText) {
		if (scanner) {
			scanner.stop().then(() => {
				scanner = null;
			});
		}
		status.textContent = "Looking up ISBN: " + decodedText + "...";
		fetch("/api/isbn/" + encodeURIComponent(decodedText)).then((resp) => {
			if (!resp.ok) throw new Error("Lookup failed");
			return resp.json();
		}).then((data) => {
			const titleEl = document.getElementById("title");
			const authorEl = document.getElementById("author");
			const isbnEl = document.getElementById("isbn");
			const coverEl = document.getElementById("cover_url");
			if (titleEl) titleEl.value = data.title;
			if (authorEl) authorEl.value = data.author;
			if (isbnEl) isbnEl.value = data.isbn;
			if (coverEl && data.cover_url) coverEl.value = data.cover_url;
			modal.classList.add("hidden");
		}).catch(() => {
			status.textContent = "Could not find book for ISBN: " + decodedText;
			setTimeout(() => {
				status.textContent = "Point your camera at the book's barcode";
				startScanner();
			}, 2e3);
		});
	}
	function closeModal() {
		if (scanner) {
			scanner.stop().then(() => {
				scanner = null;
			}).catch(() => {});
		}
		modal.classList.add("hidden");
	}
	scanBtn.addEventListener("click", () => {
		if (typeof Html5Qrcode === "undefined") {
			alert("Scanner not available. Please enter the book details manually.");
			return;
		}
		modal.classList.remove("hidden");
		startScanner();
	});
	closeBtn.addEventListener("click", closeModal);
	modal.addEventListener("click", (e) => {
		if (e.target === modal) closeModal();
	});
}
document.addEventListener("DOMContentLoaded", initScanner);
export {};
