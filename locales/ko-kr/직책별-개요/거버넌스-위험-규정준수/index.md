# 거버넌스, 위험, 규정 준수

> 엔지니어링 브리핑이며, 법률 자문, 인증, 감사가 아닙니다. 2026-10-11 현재 저장소가 보여 주는
> 내용을 서술합니다. 여기의 어떤 내용도 인증 취득을 주장하지 않습니다.

## 시작할 곳

증거 묶음은 [spec/compliance](../../../../spec/compliance/index.md)입니다. 그 기본 원칙은 팀에 지키게 해야 할 것입니다.
통제는 코드나 구성이 이를 집행하고 테스트나 검사가 이를 고정할 때에만 **Implemented**이고, 귀 조직이 제공해야 하면
**Deployer**이며, 오늘 둘 다 아니면 **Gap**입니다.

| 문서 | 용도 |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR 제32조. 각각 Implemented, Deployer, Gap으로 표시 |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | 데이터 보호 영향 평가. 이 시스템의 데이터 흐름이 미리 채워져 있음 |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | 강화, 백업과 복원, 키, 로깅, 사고 대응 |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | 제조업체가 채택할 NHS DTAC 성명 초안 |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | 영국 정부 API 표준에 대한 자체 검토(초안이며 공표된 본문과 대조하지 않음) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | 오류 형식(RFC 9457), 호환성을 깨는 변경으로 보는 기준, 지원 중단 단계와 예고 기간 |
| [NHS DTAC v2.0 평가](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | 질문별 결과: 아마도 DTAC 범위 밖이며, 조직적 증거가 없어 적용되는 항목에서 불합격할 것임 |
| [plan](../../../../spec/plan.md) | 준비 상태 판정(준비되지 않음)과 공백 목록 |
| [tasks](../../../../spec/tasks.md) | 각 공백의 작업, 해결, 명시된 한계 |

## 거버넌스

- **의사 결정권:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). 기여 및 AI 지원 규칙은 [CONTRIBUTING.md](../../../../CONTRIBUTING.md)와
  [AI_STATEMENT.md](../../../../AI_STATEMENT.md)에 있습니다.
- **변경 통제:** 명세가 기준 원천이며, 코드와 테스트가 이를 인용합니다. CI는 두 제공자에서 실행되고
  `scripts/ci-check.sh`가 이를 재현합니다. 저장소의 규칙: 무언가를 통과시키려고 테스트나 관문을 약화하지 않습니다.
- **공개:** [SECURITY.md](../../../../SECURITY.md)가 신고 경로입니다.

## 위험

위험 관점은 [CRO 개요](../최고위험책임자/index.md)에 있습니다. 요약하면, 가장 큰 잔여 위험은 외부인이 테스트하지 않은
보안(T42), 정량화되지 않은 접근성(T48), 그리고 도입자가 판단할 몫인 데이터 분류입니다.

## 규정 준수: 실제인 것과 아닌 것

| 영역 | 입장 |
|---|---|
| 접근 제어 | 구현 및 테스트됨: 역할 기반 마스킹, 레코드 수준 검사, 관리자 전용 운영 읽기, 필수 제2인 검토를 거치는 비상 접근 |
| 감사 | 구현 및 테스트됨: 해시 체인, 추가 전용, 트랜잭션 내, 목록, 거래 목록, 인사이트를 포함한 읽기 감사. **Deployer:** 체인 헤드를 데이터베이스 밖에 고정 |
| 개인정보 권리 | 구현됨: 정보 주체 열람 내보내기, 삭제(의무가 열려 있는 동안은 거부), 보존 정리. **Deployer:** 적법 근거, 보존 일정, 처리 기록 |
| 암호화 | 부분적: 로컬 키로 두 열에 필드 수준 AES-256-GCM, KMS 없음. 저장 시 및 전송 중 암호화는 배포자의 몫 |
| 공급망 | 구현됨: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, 다이제스트로 고정한 기본 이미지, Dependabot 업데이트. **Gap:** 서명된 릴리스 워크플로가 실제 릴리스에서 실행된 적 없음 (T49) |
| 접근성 | 부분적: 자동 axe 검사와 정직한 공개 성명. **Gap:** 수동 감사 없음(T48), 번역 미검토 |
| 독립적인 보증 | **Gap:** 없음 (T42) |
| 이메일 및 마케팅 법규 | 동의를 우선하는 발송 경로는 구현됨. 실제 발송은 개발용 SMTP 경로로만 존재. 수신 거부 링크, 수신 거부 목록, 서명된 결과 웹훅은 있으나 통합된 제공자가 없고 처리자 계약과 법률 검토는 열려 있음 (T39) |

## 귀 조직에 담당자가 필요한 통제

1. 비상 접근 부여의 검토와 경보(`break_glass_granted`에 경보).
2. 감사 체인 헤드를 고정하고 일정에 따라 검증(`GET /api/audits/verify`).
3. 백업과 복원의 증거, 그리고 키 교체 리허설.
4. 신원 제공자에서의 입사, 이동, 퇴사 처리.
5. 읽기 트래픽에 따라 커지는 감사 테이블의 보존.
6. Redis를 데이터베이스처럼 보호: 캐시된 인사이트 데이터를 보관합니다.
7. Dependabot 업데이트 병합, 그리고 배포 전에 서명된 릴리스 검증.

## 승인을 거부해야 할 것

시스템이 규정을 준수한다, 인증되었다, HIPAA를 준수한다, NHS가 승인했다는 모든 진술. 방어할 수 있는 진술은 더 좁습니다.
통제를 매핑했고, 구현된 것은 테스트했으며, 공백은 이름을 붙여 추적한다는 것입니다.
