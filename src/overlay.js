const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const overlay = document.getElementById("overlay");
const title = document.getElementById("overlay-title");
const detail = document.getElementById("overlay-detail");
const cancel = document.getElementById("overlay-cancel");

async function localizeCancelWhenReady() {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      const config = await invoke("get_config");
      const english = config.ui_language === "en" || (config.ui_language === "system" && !navigator.language.toLowerCase().startsWith("zh"));
      cancel.innerHTML = english ? "Esc&nbsp;&nbsp;Cancel" : "Esc&nbsp;&nbsp;取消";
      cancel.setAttribute("aria-label", english ? "Cancel" : "取消");
      return;
    } catch (error) {
      if (!String(error).includes("state not managed")) return;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
}

localizeCancelWhenReady();

listen("overlay-status", ({ payload }) => {
  overlay.dataset.state = payload.state;
  const level = Math.max(0.08, Math.min(1, payload.level || 0.08));
  overlay.style.setProperty("--level-height", `${6 + level * 22}px`);
  title.textContent = payload.title;
  detail.textContent = payload.detail || "";
});

cancel.addEventListener("click", () => invoke("cancel_current"));
