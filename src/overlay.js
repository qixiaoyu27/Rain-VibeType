const { listen } = window.__TAURI__.event;
const overlay = document.getElementById("overlay");
const title = document.getElementById("overlay-title");
const detail = document.getElementById("overlay-detail");
const bars = [...overlay.querySelectorAll(".overlay-icon i")];
const levelHistory = Array(bars.length).fill(0);

function normalizedAudioLevel(level) {
  return Math.min(1, Math.sqrt(Math.max(0, Number(level) || 0)) * 2.4);
}

console.assert(normalizedAudioLevel(-1) === 0 && normalizedAudioLevel(1) === 1);

function syncTheme() {
  document.documentElement.dataset.theme = localStorage.getItem("rain-theme") || "light";
}

syncTheme();
window.addEventListener("storage", syncTheme);

listen("overlay-status", ({ payload }) => {
  syncTheme();
  overlay.dataset.state = payload.state;
  overlay.style.setProperty("--overlay-opacity", String(payload.opacity ?? 0.68));
  if (payload.state === "recording") {
    const level = normalizedAudioLevel(payload.level);
    levelHistory.push(level);
    levelHistory.shift();
    bars.forEach((bar, index) => {
      bar.style.height = `${6 + levelHistory[index] * 24}px`;
      bar.style.opacity = String(.55 + levelHistory[index] * .45);
    });
  } else {
    levelHistory.fill(0);
    bars.forEach((bar) => {
      bar.style.height = "5px";
      bar.style.opacity = "1";
    });
  }
  title.textContent = payload.title;
  detail.textContent = payload.detail || "";
});
