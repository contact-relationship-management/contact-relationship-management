# Dành cho Giám đốc an ninh thông tin

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Tư thế bảo mật trong một đoạn

Dịch vụ **dừng an toàn khi lỗi**, kiểm soát truy cập theo vai trò và theo từng bản ghi, che các giá trị nhạy cảm với những
vai trò không nên thấy, và giữ một vết kiểm toán cho thấy dấu hiệu giả mạo. Các biện pháp kiểm soát đó do các tác giả
triển khai và tự kiểm thử. Chúng **chưa được ai độc lập kiểm thử** (T42), vì vậy hãy xem mọi phát biểu dưới đây là
"đã xây dựng và tự kiểm thử", không phải "đã xác minh".

## Các biện pháp kiểm soát và nơi kiểm tra

| Lĩnh vực | Những gì đã có | Xem ở đâu |
|---|---|---|
| Xác thực | PASETO v4.public xác minh ngoại tuyến, Keycloak tùy chọn; môi trường sản xuất từ chối khởi động khi xác thực bị tắt | [auth](../../../../spec/auth.md) |
| Phân quyền | Dựa trên thuộc tính: đọc, ghi, xóa, hủy hoại; nghĩa vụ `mask`; kiểm tra ở cấp bản ghi; kiểm toán, lưu giữ và xem xét break-glass chỉ dành cho quản trị viên | [auth](../../../../spec/auth.md), kiểm thử ma trận persona |
| Lộ dữ liệu | Che trên bản ghi, tổng hợp và insight; một lỗ rò rỉ ở chi tiết tài khoản đã được tìm thấy và sửa (T47) | [tasks](../../../../spec/tasks.md) |
| Truy cập khẩn cấp | Lý do bằng văn bản, giới hạn 60 phút, chỉ đọc, bắt buộc người thứ hai xem xét, được kiểm toán, chia sẻ qua Redis | các tác vụ break-glass T44 |
| Kiểm toán | Chuỗi hash SHA-256 trong cùng giao dịch, trigger chỉ ghi thêm, điểm cuối xác minh; các lần đọc được kiểm toán | [audit](../../../../spec/audit.md) |
| Khả năng chống lạm dụng | Giới hạn tốc độ (600 mỗi phút) và khóa (10 lần thất bại trong 5 phút), chia sẻ qua Redis; các probe được miễn | T43 |
| Trình duyệt và truyền tải | CSP nghiêm ngặt với hash, chặn khung, `nosniff`, chính sách referrer, CORS đóng, các header an toàn; không có token trong JavaScript của trình duyệt (BFF) | UI `hooks.server.ts` |
| Bảo vệ dữ liệu | AES-256-GCM trên hai cột có xoay khóa; khóa cục bộ, không có KMS | T41 |
| Chuỗi cung ứng | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, image đã gia cố không chạy bằng root được ghim theo digest, cập nhật Dependabot hằng tuần, một quy trình phát hành ký image và SBOM không cần khóa | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Lỗi | Mọi lỗi là một tài liệu vấn đề RFC 9457; một `401` nêu tên lược đồ được chấp nhận | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| An toàn bộ nhớ | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Các điểm yếu đã biết (nêu rõ, không giấu)

- **Chưa có kiểm thử xâm nhập độc lập hay buổi mô hình hóa mối đe dọa** (T42). Mô hình
  mối đe dọa trong [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  là một bản tóm tắt, không phải mô hình có cấu trúc.
- **Việc ký bản phát hành chưa chạy trên bản phát hành thật** (T49). Quy trình và các
  lệnh xác minh đã có; cho đến khi một thẻ được đẩy lên thì chúng chưa được kiểm thử. Các action
  được ghim theo hash commit.
- **Mã hóa trường** bao phủ hai cột với khóa cục bộ, không có KMS.
- **Xác thực lại từng bước** trước các hành động hủy hoại chưa được xây dựng.
- **Redis giữ dữ liệu cá nhân** (insight được lưu đệm, khóa quyền) và phải được bảo vệ
  như cơ sở dữ liệu: TLS, mật khẩu, không mở công khai.
- **Break-glass** không thể thu hồi, chỉ có thể hết hạn, và chưa nối cảnh báo nào.
- **Giới hạn tốc độ và khóa** quay về đếm theo từng bản sao nếu Redis hỏng; nhóm
  không xác thực tính theo địa chỉ, và sau một proxy không có trong
  `TRUSTED_PROXIES` thì đó là của proxy, nên kẻ tấn công có thể khóa những người gọi
  không xác thực khác. Người gọi có token hợp lệ được đếm theo chủ thể của nó.
- **Việc tải lại chính sách** xuất hiện trong các insight được lưu đệm sau tối đa 30 giây.
- **Đăng xuất khi không hoạt động** được thực hiện ở phía máy khách.
- **Khóa idempotency** là tùy chọn ở API (T52): một máy khách không gửi
  `Idempotency-Key` không được bảo vệ khỏi việc một lần tạo được thử lại làm trùng một bản ghi.
  Giao diện đi kèm có gửi một khóa.

## Bên triển khai phải làm gì

1. Giữ xác thực luôn bật; cung cấp các khóa sản xuất và `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Cảnh báo trên `break_glass_granted` và xem xét các quyền được cấp kịp thời.
3. Neo đầu chuỗi kiểm toán bên ngoài cơ sở dữ liệu và xác minh nó
   (`GET /api/audits/verify`) theo lịch.
4. Bảo vệ PostgreSQL và Redis (mạng, TLS, thông tin xác thực, sao lưu, mã hóa
   khi lưu trữ); xoay khóa trường bằng tác vụ được cung cấp.
5. Gộp các cập nhật Dependabot; chạy lại giai đoạn chuỗi cung ứng; xác minh một bản
   phát hành đã ký trước khi triển khai.
6. Làm theo sổ tay để xử lý sự cố
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Xử lý lỗ hổng

Báo cáo đi qua [SECURITY.md](../../../../SECURITY.md). Hãy đặt hàng
kiểm thử độc lập (T42) trên một bản staging trước khi có bất kỳ dữ liệu thật nào, và
kiểm thử lại sau mỗi thay đổi đáng kể.

## Các hành động bảo mật đề xuất, theo thứ tự

1. Xác định phạm vi và đặt hàng kiểm thử xâm nhập độc lập (T42).
2. Đẩy một thẻ phát hành đầu tiên để chứng minh việc ký và xác minh (T49).
3. Nối cảnh báo cho break-glass và cho các lần xác minh chuỗi kiểm toán thất bại.
4. Đặt Redis sau TLS và thông tin xác thực ở mọi môi trường.
5. Khóa trường dựa trên KMS và phạm vi trường rộng hơn (phần còn lại của T41).

Xem thêm bản tóm tắt cho [Giám đốc công nghệ](../giám-đốc-công-nghệ/index.md) và
[Giám đốc rủi ro](../giám-đốc-rủi-ro/index.md).
