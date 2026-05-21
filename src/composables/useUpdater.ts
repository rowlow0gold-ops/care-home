/**
 * Wrapper around Tauri's updater plugin. Call from a menu item or settings page.
 *
 *   const { checkAndInstall } = useUpdater()
 *   await checkAndInstall()  // prompts user, downloads, restarts
 *
 * The server endpoint shape is configured in tauri.conf.json -> plugins.updater.
 * It must return JSON like:
 *   { "version": "0.2.0",
 *     "notes": "release notes",
 *     "pub_date": "2026-06-01T00:00:00Z",
 *     "platforms": {
 *       "windows-x86_64": { "signature": "...", "url": "https://..../care-home.msi" }
 *     }
 *   }
 *
 * For now the endpoint returns 204 (no update) until we wire up
 * care-home-server /api/v1/updates/desktop/*.
 */
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export function useUpdater() {
  async function checkAndInstall(opts: { silent?: boolean } = {}) {
    const update = await check();
    if (!update) {
      if (!opts.silent) {
        // eslint-disable-next-line no-alert
        alert("최신 버전을 사용 중입니다.");
      }
      return false;
    }

    if (!opts.silent) {
      const ok = window.confirm(
        `새 버전 ${update.version} 이 사용 가능합니다.\n\n${update.body ?? ""}\n\n지금 설치하시겠습니까?`,
      );
      if (!ok) return false;
    }

    await update.downloadAndInstall();
    await relaunch();
    return true;
  }

  return { checkAndInstall };
}
