# 治理、風險、合規

> 工程簡報，不是法律意見、認證或稽核。內容反映截至 2026-10-11 程式碼儲存庫所顯示的情況。本文不聲稱已獲得任何認可。

## 從哪裡開始

證據包位於 [spec/compliance](../../../../spec/compliance/index.md)。它的基本規則也是應當要求團隊遵守的規則：只有當程式碼或配置強制執行、並且有測試或檢查固定住它時，控制才算**已實現**；當必須由您的組織提供時為**部署方**；兩者都不成立時為**差距**。

| 文件 | 用途 |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53、HIPAA 安全規則、ISO 27001 附錄 A、GDPR 第 32 條，每項均標為已實現、部署方或差距 |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | 資料保護影響評估，已按本系統的資料流預先填寫 |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | 加固、備份與恢復、金鑰、記錄、事件回應 |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | 供製造商採用的 NHS DTAC 聲明草稿 |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | 對照英國政府 API 標準的自我評審（草稿，未對照已發布的文本核對） |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | 錯誤格式（RFC 9457）、什麼算破壞性變更、棄用步驟和通知期 |
| [NHS DTAC v2.0 評估](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | 逐題結果：很可能不在 DTAC 範圍內，並且因缺乏組織層面的證據，在適用的部分不會透過 |
| [plan](../../../../spec/plan.md) | 就緒度結論（尚未就緒）和差距登記冊 |
| [tasks](../../../../spec/tasks.md) | 每個差距對應的任務、解決情況和已說明的限制 |

## 治理

- **決策權：** [GOVERNANCE.md](../../../../GOVERNANCE.md)、[MAINTAINERS.md](../../../../MAINTAINERS.md)、[CODEOWNERS](../../../../CODEOWNERS)。貢獻和 AI 輔助的規則見 [CONTRIBUTING.md](../../../../CONTRIBUTING.md) 和 [AI_STATEMENT.md](../../../../AI_STATEMENT.md)。
- **變更控制：** 規範是權威來源；程式碼和測試都引用規範。CI 在兩個平臺上執行，`scripts/ci-check.sh` 可復現它。儲存庫中的規則：不要為了讓某項通過而削弱測試或關卡。
- **披露：** [SECURITY.md](../../../../SECURITY.md) 是報告管道。

## 風險

風險檢視見[風險長簡報](../風險長/index.md)。簡言之：剩餘風險最高的是未經外部檢驗的安全性（T42）、無法量化的無障礙狀況（T48），以及資料分類——這由採用方決定。

## 合規：哪些是真實的，哪些不是

| 領域 | 現狀 |
|---|---|
| 存取控制 | 已實現並經過測試：基於角色的遮罩、記錄級檢查、僅限管理員的運營讀取、強制由第二人審查的破玻璃 |
| 稽核 | 已實現並經過測試：雜湊鏈、僅追加、在事務內寫入，讀取稽核涵蓋列表、交易列表和洞察。**部署方：** 在資料庫之外錨定鏈頭 |
| 隱私權利 | 已實現：主體存取匯出、刪除（存在未了結義務時被拒絕）、保留清理。**部署方：** 合法依據、保留計劃、處理活動記錄 |
| 加密 | 部分：兩列使用本地金鑰的欄位級 AES-256-GCM，沒有 KMS。靜態加密和傳輸加密由部署方負責 |
| 供應鏈 | 已實現：`cargo audit`、`cargo deny`、`pnpm audit`、SBOM、按摘要固定的基礎映象、Dependabot 更新。**差距：** 已簽名發布的工作流尚未在真實發布上執行（T49） |
| 無障礙 | 部分：自動 axe 掃描和如實的公開聲明。**差距：** 沒有人工稽核（T48），譯文未經審閱 |
| 獨立保證 | **差距：** 沒有（T42） |
| 電子郵件與行銷法規 | 以同意為先的傳送路徑已實現；真實發送僅以供開發使用的 SMTP 路徑存在；退訂連結、抑制名單和已簽名的結果回撥已存在，但尚未接入服務商，處理者協議和法律審查仍未完成（T39） |

## 需要在您的組織中指定負責人的控制措施

1. 對破玻璃授權的審查和告警（對 `break_glass_granted` 設定告警）。
2. 錨定稽核鏈頭，並按計劃驗證（`GET /api/audits/verify`）。
3. 備份與恢復的證據，以及金鑰輪換演練。
4. 在您的身份提供方中處理入職、調動和離職。
5. 稽核表的保留期限——它會隨讀取流量增長。
6. 像保護資料庫一樣保護 Redis：它儲存著快取的洞察資料。
7. 合併 Dependabot 更新，並在部署前驗證已簽名的發布。

## 應當拒絕簽署的內容

任何聲稱系統合規、已認證、符合 HIPAA 或獲 NHS 批准的說法。可以站得住腳的說法更剋制：控制措施已對映，已實現的已測試，差距已列明並跟蹤。
