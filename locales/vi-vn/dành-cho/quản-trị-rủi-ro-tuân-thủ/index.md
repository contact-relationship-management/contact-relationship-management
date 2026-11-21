# Quản trị, rủi ro, tuân thủ

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Bắt đầu từ đâu

Bộ bằng chứng là [spec/compliance](../../../../spec/compliance/index.md). Quy tắc nền
của nó là điều cần buộc đội ngũ giữ: một biện pháp kiểm soát chỉ là **Implemented** khi
mã hoặc cấu hình thực thi nó và một bài kiểm thử hoặc kiểm tra cố định nó; là **Deployer**
khi tổ chức của bạn phải cung cấp nó; là **Gap** khi hôm nay không điều nào đúng.

| Tài liệu | Dùng cho |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR Điều 32, mỗi mục được đánh dấu Implemented, Deployer hoặc Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Đánh giá tác động bảo vệ dữ liệu, điền sẵn các luồng dữ liệu của hệ thống này |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Gia cố, sao lưu và khôi phục, khóa, ghi nhật ký, xử lý sự cố |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Bản nháp các tuyên bố NHS DTAC để nhà sản xuất tiếp nhận |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Tự rà soát theo các tiêu chuẩn API của chính phủ Anh (bản nháp, chưa đối chiếu với văn bản đã công bố) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Định dạng lỗi (RFC 9457), điều gì là thay đổi phá vỡ, các bước ngừng hỗ trợ và thời hạn thông báo |
| [Đánh giá NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Kết quả theo từng câu hỏi: có khả năng nằm ngoài phạm vi DTAC, và sẽ không đạt các phần áp dụng do thiếu bằng chứng ở cấp tổ chức |
| [plan](../../../../spec/plan.md) | Kết luận về mức sẵn sàng (chưa sẵn sàng) và sổ đăng ký khoảng trống |
| [tasks](../../../../spec/tasks.md) | Tác vụ của từng khoảng trống, cách giải quyết và các giới hạn được nêu |

## Quản trị

- **Quyền quyết định:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Quy tắc đóng góp và hỗ trợ bằng AI nằm trong
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) và [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Kiểm soát thay đổi:** đặc tả là nguồn chân lý; mã và kiểm thử
  trích dẫn nó. CI chạy trên hai nhà cung cấp, và `scripts/ci-check.sh` tái tạo nó.
  Quy tắc trong kho mã: đừng làm yếu một bài kiểm thử hay cổng để cho một thứ vượt qua.
- **Công bố:** [SECURITY.md](../../../../SECURITY.md) là đường báo cáo.

## Rủi ro

Cái nhìn về rủi ro nằm trong bản tóm tắt [Giám đốc rủi ro](../giám-đốc-rủi-ro/index.md). Tóm lại: các rủi ro còn lại
cao nhất là bảo mật chưa được người ngoài kiểm thử (T42), khả năng truy cập chưa
định lượng (T48), và phân loại dữ liệu, vốn là quyết định của bên áp dụng.

## Tuân thủ: điều gì là thật và điều gì không

| Lĩnh vực | Vị thế |
|---|---|
| Kiểm soát truy cập | Đã triển khai và kiểm thử: che theo vai trò, kiểm tra ở cấp bản ghi, các lần đọc vận hành chỉ dành cho quản trị viên, break-glass với việc bắt buộc người thứ hai xem xét |
| Kiểm toán | Đã triển khai và kiểm thử: xâu chuỗi hash, chỉ ghi thêm, trong giao dịch, kiểm toán việc đọc gồm cả danh sách, danh sách giao dịch và insight. **Deployer:** neo đầu chuỗi bên ngoài cơ sở dữ liệu |
| Quyền riêng tư | Đã triển khai: xuất dữ liệu để truy cập dữ liệu cá nhân, xóa (bị từ chối khi còn nghĩa vụ mở), quét lưu giữ. **Deployer:** cơ sở pháp lý, lịch lưu giữ, hồ sơ xử lý |
| Mã hóa | Một phần: AES-256-GCM cấp trường trên hai cột với khóa cục bộ, không có KMS. Mã hóa khi lưu trữ và khi truyền thuộc về bên triển khai |
| Chuỗi cung ứng | Đã triển khai: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, image cơ sở ghim theo digest, cập nhật Dependabot. **Gap:** quy trình phát hành đã ký chưa chạy trên bản phát hành thật (T49) |
| Khả năng truy cập | Một phần: các lần quét axe tự động và một tuyên bố công khai trung thực. **Gap:** chưa có kiểm toán thủ công (T48), bản dịch chưa được xem xét |
| Bảo đảm độc lập | **Gap:** không có (T42) |
| Luật email và tiếp thị | Đường gửi đặt sự đồng ý lên trước đã triển khai; việc gửi thật chỉ tồn tại dưới dạng một đường SMTP cho phát triển; liên kết hủy đăng ký, danh sách chặn và một webhook kết quả đã ký đã có, nhưng chưa tích hợp nhà cung cấp nào và thỏa thuận bên xử lý cùng rà soát pháp lý còn mở (T39) |

## Các biện pháp kiểm soát cần một người chịu trách nhiệm trong tổ chức của bạn

1. Xem xét và cảnh báo cho các quyền break-glass (cảnh báo trên `break_glass_granted`).
2. Neo đầu chuỗi kiểm toán và xác minh nó theo lịch
   (`GET /api/audits/verify`).
3. Bằng chứng sao lưu và khôi phục, và diễn tập xoay khóa.
4. Xử lý nhân sự vào, chuyển, rời trong nhà cung cấp định danh của bạn.
5. Lưu giữ cho bảng kiểm toán, vốn lớn lên theo lưu lượng đọc.
6. Bảo vệ Redis như cơ sở dữ liệu: nó giữ dữ liệu insight được lưu đệm.
7. Gộp các cập nhật Dependabot, và xác minh một bản phát hành đã ký trước khi triển khai.

## Điều gì nên từ chối phê duyệt

Mọi tuyên bố rằng hệ thống tuân thủ, được chứng nhận, tuân thủ HIPAA hay được NHS
phê duyệt. Tuyên bố có thể bảo vệ được hẹp hơn: các biện pháp kiểm soát đã được ánh xạ, những biện pháp
đã triển khai được kiểm thử, và các khoảng trống được nêu tên và theo dõi.
