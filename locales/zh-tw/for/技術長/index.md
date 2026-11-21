# 致技術長

> 工程簡報，不是法律意見、認證或稽核。內容反映截至 2026-10-11 程式碼儲存庫所顯示的情況。本文不聲稱已獲得任何認可。

## 架構

- **服務：** 基於 axa（Axum、sqlx 0.9）的 Rust，PostgreSQL 18，Redis 用於快取、共享計數器和破玻璃授權的傳播。`forbid(unsafe_code)`，clippy pedantic。行為的權威來源是規範（[index](../../../../spec/index.md)、[architecture](../../../../spec/architecture.md)）。
- **UI：** SvelteKit 3 / Svelte 5，採用後端即前端（BFF）代理，因此權杖絕不會進入瀏覽器 JavaScript；使用帶雜湊的嚴格 CSP。
- **認證：** 離線驗證的 PASETO v4.public，可選 Keycloak；基於屬性的存取控制（讀/寫/刪除/破壞性操作、`mask` 義務、記錄級檢查），來自共享引擎。
- **稽核：** SHA-256 雜湊鏈，與變更在同一事務中寫入，帶僅追加觸發器，可透過 `GET /api/audits/verify` 驗證。
- **資料保護：** 兩列使用 AES-256-GCM 欄位加密並支援金鑰輪換；金鑰為本地金鑰，沒有 KMS。

## 品質證據

單元測試（176 個）、資料庫測試（41 個，包括強制執行、遮罩、稽核、分頁、快取、冪等、模式校驗）、前端單元測試（96 個）以及 Playwright（85 個），其中包含 axe 無障礙掃描。CI 執行 fmt、clippy、測試、前端建置/lint，以及供應鏈階段（`cargo audit`、`cargo deny`、`pnpm audit`、CycloneDX SBOM），執行於 GitHub Actions 和 Woodpecker。一個指令即可復現：`scripts/ci-check.sh`。

## 技術債務與如實說明的限制

- 分頁使用偏移量游標（如果遍歷期間資料發生變化，行可能被跳過或重複）；按設計不提供總數。
- 冪等鍵在 API 層是可選啟用的（T52）：客戶端必須在建立請求上傳送 `Idempotency-Key`，才能避免重試造成重複記錄。隨附的 UI 在每個建立請求上都會發送。
- OpenAPI 結構描述是手寫的，涵蓋請求、列表、詳細資料讀取、儀表板、洞察檢視、稽核、隱私和破玻璃；傳回更新後記錄的狀態轉換和 upsert 尚無型別。路由與文件不一致時測試會失敗，另有測試用真實回應（包括被遮罩的回應）驗證這些結構描述（[gds-api-review](../../../../spec/compliance/gds-api-review.md)）。
- 錯誤是 RFC 9457 問題文件（T54），並保留原有的 `error` 和 `description` 成員；路由器之外的層（大小限制、超時）產生的錯誤不會被轉換。版本管理和棄用政策見 [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)；`Deprecation` 和 `Sunset` 頭已有規定但尚未建置。
- 欄位加密僅覆蓋兩列；沒有整合 KMS。
- 限流和快取狀態需要 Redis 才能在多副本下保持正確。
- 洞察檢視按角色遮罩，但按呼叫者權杖快取 30 秒。
- 破壞性操作前的升級再認證（step-up）需要身份提供方配合。

## 建議的技術順序

1. 由 KMS 支援的欄位金鑰以及更廣的欄位覆蓋（T41 的剩餘部分）。
2. 打第一個發布標籤，以驗證無金鑰簽名和驗證流程（T49），並請評審人核對 API 標準草稿。
3. 在第一次棄用時建置 `Deprecation` 和 `Sunset` 頭。
4. 針對預發布（staging）建置委託獨立測試（T42）。
