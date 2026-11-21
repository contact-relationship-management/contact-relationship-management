# Dành cho Giám đốc công nghệ

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Kiến trúc

- **Dịch vụ:** Rust trên Axum và axa (sqlx 0.9), PostgreSQL 18, Redis cho bộ nhớ đệm,
  bộ đếm dùng chung và lan truyền break-glass. `forbid(unsafe_code)`, clippy
  pedantic. Nguồn chân lý về hành vi là đặc tả
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Giao diện:** SvelteKit 3 / Svelte 5, một proxy backend-for-frontend để không token nào
  chạm tới JavaScript của trình duyệt, CSP nghiêm ngặt với hash.
- **Xác thực:** PASETO v4.public xác minh ngoại tuyến, Keycloak tùy chọn; kiểm soát truy cập
  dựa trên thuộc tính (các hành động đọc/ghi/xóa/hủy hoại, nghĩa vụ `mask`,
  kiểm tra ở cấp bản ghi) từ một engine dùng chung.
- **Kiểm toán:** chuỗi hash SHA-256 ghi trong cùng giao dịch với thay đổi,
  trigger chỉ ghi thêm, có thể xác minh tại `GET /api/audits/verify`.
- **Bảo vệ dữ liệu:** mã hóa trường AES-256-GCM có xoay khóa trên hai
  cột; khóa cục bộ, không có KMS.

## Bằng chứng về chất lượng

Kiểm thử đơn vị (185), kiểm thử cơ sở dữ liệu (56, gồm thực thi, che, kiểm toán,
phân trang, lưu đệm, idempotency, kiểm tra lược đồ), kiểm thử đơn vị front-end (106) và
Playwright (98) gồm cả các lần quét khả năng truy cập axe. CI chạy fmt, clippy, kiểm thử, build/lint front-end và một
giai đoạn chuỗi cung ứng (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM CycloneDX)
trên GitHub Actions và Woodpecker. Một lệnh tái tạo tất cả: `scripts/ci-check.sh`.

## Nợ kỹ thuật và các giới hạn trung thực

- Phân trang dùng con trỏ offset (các dòng có thể bị bỏ qua hoặc lặp lại nếu dữ liệu thay đổi
  giữa chừng); cố ý không có tổng số.
- Khóa idempotency là tùy chọn ở API (T52): một máy khách phải gửi
  `Idempotency-Key` khi tạo để được bảo vệ khỏi một lần thử lại làm trùng
  bản ghi. Giao diện đi kèm gửi một khóa ở mọi lần tạo.
- Lược đồ OpenAPI được viết tay. Chúng bao phủ yêu cầu, danh sách, đọc chi tiết,
  bảng điều khiển, màn hình insight, kiểm toán, quyền riêng tư và break-glass; chuyển trạng thái
  và upsert trả về bản ghi đã cập nhật thì chưa định kiểu. Một bài kiểm thử thất bại khi
  một route và tài liệu không khớp, và các bài khác kiểm tra phản hồi thật, kể cả phản hồi bị che,
  so với các lược đồ
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Lỗi là các tài liệu vấn đề RFC 9457 (T54) vẫn giữ các thành viên `error` và
  `description` cũ; lỗi từ các tầng ngoài router (giới hạn kích thước,
  hết thời gian) không được chuyển đổi. Chính sách đánh phiên bản và ngừng hỗ trợ là
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); các header
  `Deprecation` và `Sunset` đã được đặc tả nhưng chưa xây dựng.
- Mã hóa trường bao phủ hai cột; chưa tích hợp KMS.
- Trạng thái giới hạn tốc độ và bộ nhớ đệm cần Redis để đúng khi có nhiều bản sao.
- Các màn hình insight được che theo vai trò nhưng được lưu đệm theo token của người gọi trong 30 giây.
- Xác thực lại từng bước trước các hành động hủy hoại cần công việc ở nhà cung cấp định danh.

## Trình tự kỹ thuật khuyến nghị

1. Khóa trường dựa trên KMS và phạm vi trường rộng hơn (phần còn lại của T41).
2. Gắn thẻ một bản phát hành đầu tiên để chứng minh việc ký và xác minh không cần khóa (T49), và
   nhờ một người rà soát kiểm tra bản nháp tiêu chuẩn API.
3. Xây dựng các header `Deprecation` và `Sunset` cùng với lần ngừng hỗ trợ đầu tiên.
4. Đặt hàng kiểm thử độc lập (T42) trên một bản staging.
