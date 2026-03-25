import { WebHaptics } from "web-haptics";

const haptics = new WebHaptics();

// Three haptic tiers:
//   .haptic-heavy  — primary actions (log book, read again): solid thunk
//   .haptic-medium — secondary actions (nav tabs, toggles, save, scan): firm tick
//   .haptic-light  — navigation (book cards, pagination, sort): subtle tap
document.addEventListener(
  "touchstart",
  (e) => {
    const t = e.target as Element;
    if (!t.closest) return;
    if (t.closest(".haptic-heavy")) {
      haptics.trigger("heavy");
      return;
    }
    if (t.closest(".haptic-medium")) {
      haptics.trigger("medium");
      return;
    }
    if (t.closest(".haptic-light")) {
      haptics.trigger("light");
      return;
    }
  },
  { passive: true },
);
