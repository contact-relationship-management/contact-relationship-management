# Dành cho Giám đốc vận hành

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Vận hành cái này cần gì

Dịch vụ là một tiến trình Rust cho mỗi bản sao, cùng PostgreSQL 18 và Redis; phần giao diện là một tiến trình Node. Cả
hai được phát hành dưới dạng image container đã được gia cố, không chạy bằng root, với các image cơ sở được ghim theo
digest. Sổ tay vận hành là
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Những sự thật vận hành cần lập kế hoạch

- **Lỗi thì dừng an toàn khi khởi động.** Trong môi trường sản xuất, dịch vụ từ chối khởi động nếu xác thực bị tắt, nếu
  chuỗi kiểm toán không dùng được, hoặc nếu thiếu các khóa bắt buộc và
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`. Một bản triển khai cấu hình sai sẽ dừng thay vì chạy mở.
- **Sức khỏe và số liệu.** `/_health/live`, `/_health/ready`, `/_health/startup` không bao giờ bị giới hạn tốc độ, nên
  một lần khóa không thể làm khởi động lại một pod đang khỏe. Số liệu ở
  `/metrics.prom`.
- **Giới hạn chống lạm dụng** được bật trong môi trường sản xuất: 600 yêu cầu mỗi phút cho mỗi máy khách và khóa sau 10
  lần đăng nhập thất bại trong 5 phút. Chúng được chia sẻ giữa các bản sao qua Redis; nếu Redis hỏng, chúng quay về đếm
  theo từng bản sao.
- **Truy cập khẩn cấp** (break-glass): bất kỳ người dùng đã đăng nhập nào cũng có thể nâng quyền đọc có lý do trong tối
  đa 60 phút. Một người khác phải xem xét mọi quyền được cấp.
  **Quy trình bạn phải chạy:** cảnh báo trên các dòng kiểm toán `break_glass_granted` và
  xem xét số lượng chưa được xem xét (trang quản trị `/break-glass/review`).
- **Lỗi** có một dạng duy nhất (RFC 9457 `application/problem+json`, vẫn giữ `error` và
  `description`), nên việc phân tích nhật ký và các quy tắc cảnh báo có thể dựa vào `status` và `type`.
- **Kiểm toán tăng dần.** Các lần đọc được kiểm toán, nên bảng kiểm toán lớn lên theo lưu lượng.
  Hãy định cỡ việc lưu giữ cho nó. Một máy khách thăm dò liên tục sẽ làm nó lớn rất nhanh.

## Công việc vận hành còn mở

| Hạng mục | Trạng thái |
|---|---|
| Diễn tập sao lưu/khôi phục, diễn tập xoay khóa | Đã viết trong sổ tay; **bạn** phải chạy và lưu bằng chứng |
| Neo đầu chuỗi kiểm toán bên ngoài cơ sở dữ liệu | Nghĩa vụ của bạn; API báo cáo hash đầu chuỗi |
| Làm mới digest image đã ghim | Dependabot mở các pull request hằng tuần; **bạn** phải xem xét và gộp chúng, nếu không các bản ghim sẽ cũ đi |
| Xác minh bản phát hành đã ký trước khi triển khai | Các lệnh nằm trong sổ tay; triển khai theo digest và từ chối image không qua xác minh. Chưa được kiểm thử cho đến khi có bản phát hành được gắn thẻ đầu tiên |
| Nhà cung cấp email thật (T39) | Còn mở; thay đổi bức tranh sự cố và nhà cung cấp |
| Redis giữ dữ liệu insight | Bảo vệ nó như cơ sở dữ liệu: TLS, mật khẩu, không mở công khai |

## Các giới hạn đã biết nên nói với đội trực

Đăng xuất khi không hoạt động được thực hiện ở phía máy khách; một token bị thu hồi bị từ chối ngay, nhưng việc tải lại
chính sách xuất hiện trong các insight được lưu đệm sau tối đa 30 giây; break-glass không thể
thu hồi, chỉ có thể hết hạn.
