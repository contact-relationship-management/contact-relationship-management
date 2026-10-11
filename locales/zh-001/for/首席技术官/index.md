# 致首席技术官

> 工程简报，不是法律意见、认证或审计。内容反映截至 2026-10-11 代码仓库所显示的情况。本文不声称已获得任何认可。

## 架构

- **服务：** 基于 Loco（Axum、SeaORM 2.0）的 Rust，PostgreSQL 18，Redis 用于缓存、共享计数器和破玻璃授权的传播。`forbid(unsafe_code)`，clippy pedantic。行为的权威来源是规范（[index](../../../../spec/index.md)、[architecture](../../../../spec/architecture.md)）。
- **UI：** SvelteKit 2 / Svelte 5，采用后端即前端（BFF）代理，因此令牌绝不会进入浏览器 JavaScript；使用带哈希的严格 CSP。
- **认证：** 离线验证的 PASETO v4.public，可选 Keycloak；基于属性的访问控制（读/写/删除/破坏性操作、`mask` 义务、记录级检查），来自共享引擎。
- **审计：** SHA-256 哈希链，与变更在同一事务中写入，带仅追加触发器，可通过 `GET /api/audits/verify` 验证。
- **数据保护：** 两列使用 AES-256-GCM 字段加密并支持密钥轮换；密钥为本地密钥，没有 KMS。

## 质量证据

单元测试（116 个）、数据库测试（22 个，包括强制执行、脱敏、审计、分页、缓存）、前端单元测试（35 个）以及 Playwright（39 个），其中包含 axe 无障碍扫描。CI 运行 fmt、clippy、测试、前端构建/lint，以及供应链阶段（`cargo audit`、`cargo deny`、`pnpm audit`、CycloneDX SBOM），运行于 GitHub Actions 和 Woodpecker。一条命令即可复现：`scripts/ci-check.sh`。

## 技术债务与如实说明的限制

- 分页使用偏移量游标（如果遍历期间数据发生变化，行可能被跳过或重复）；按设计不提供总数。
- 幂等键是可选启用的（T52）：客户端必须在创建请求上发送 `Idempotency-Key`，才能避免重试造成重复记录。
- OpenAPI 是手写的，没有请求/响应模式；有路由未文档化时，构建不会失败（[gds-api-review](../../../../spec/compliance/gds-api-review.md)）。
- 错误格式是 Loco 的 `{error, description}`，而不是 RFC 9457。
- 字段加密仅覆盖两列；没有集成 KMS。
- 限流和缓存状态需要 Redis 才能在多副本下保持正确。
- 洞察视图按角色脱敏，但按调用者令牌缓存 30 秒。
- 破坏性操作前的升级再认证（step-up）需要身份提供方配合。

## 建议的技术顺序

1. OpenAPI 模式，外加路由覆盖测试。
2. 由 KMS 支持的字段密钥以及更广的字段覆盖（T41 的剩余部分）。
3. 在确定密钥保管方式后，对发布构建进行签名（T49）。
4. 针对预发布（staging）构建委托独立测试（T42）。
