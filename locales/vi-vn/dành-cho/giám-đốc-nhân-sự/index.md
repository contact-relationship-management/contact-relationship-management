# Dành cho Giám đốc nhân sự

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Vì sao con người quan trọng với sản phẩm này

Quyền truy cập dữ liệu cá nhân được quyết định theo vai trò. Phần mềm thực thi những gì token của một người nói;
**ai được vai trò nào, và khi nào vai trò bị gỡ, là một quy trình về con người** mà kho mã này không thể làm thay bạn.

## Phần mềm cung cấp gì cho quy trình của bạn

- **Khả năng nhìn thấy theo vai trò:** một số vai trò thấy các giá trị bị che cho kênh liên hệ, số tiền giao dịch và các
  màn hình tiền tệ; quản lý, quản trị viên và các cấp dịch vụ thấy nhiều hơn
  ([auth](../../../../spec/auth.md)).
- **Các lần đọc được kiểm toán:** việc đọc dữ liệu cá nhân để lại dấu vết nêu tên người đọc.
  Nhân viên nên được thông báo điều này trước khi đưa vào hoạt động.
- **Truy cập khẩn cấp:** một người có thể nâng quyền đọc có lý do tối đa 60
  phút; một người khác phải xem xét sau đó. Hãy đào tạo cả hai phía.
- **Đăng xuất khi không hoạt động** sau 15 phút trong trình duyệt.
- **Khả năng truy cập:** một tuyên bố công khai và các kiểm tra tự động; cuộc kiểm toán thủ công
  (T48) vẫn cần người khuyết tật và người kiểm thử công nghệ hỗ trợ.
- **Ngôn ngữ:** giao diện được dịch ra 8 vùng ngôn ngữ; các bản dịch **chưa** được người bản ngữ xem lại.

## Trách nhiệm của bạn

| Lĩnh vực | Hành động |
|---|---|
| Nhân sự vào, chuyển, rời | Cấp và gỡ vai trò qua nhà cung cấp định danh của bạn; CRM từ chối token hết hạn nhưng không quản lý tài khoản |
| Đào tạo | Bảo vệ dữ liệu, quy tắc đồng ý, việc dùng và xem xét break-glass, thực hành dễ tiếp cận |
| Chính sách | Sử dụng chấp nhận được, thông báo giám sát (các lần đọc được kiểm toán), đường xử lý kỷ luật khi lạm dụng truy cập khẩn cấp |
| Năng lực xem xét | Nêu tên người xem xét các quyền break-glass và người thay thế khi họ vắng mặt |
| Tuyển dụng | Các vai trò chương trình cần: đầu mối với người kiểm thử độc lập, cán bộ an toàn lâm sàng nếu thuộc phạm vi NHS, phụ trách bảo vệ dữ liệu, phụ trách khả năng truy cập |
| Phúc lợi | Hỗ trợ nhân viên xử lý các trường hợp nhạy cảm; CRM không phải hệ thống quản lý ca nhưng có thể chứa dữ liệu quan hệ nhạy cảm |

## Các hạng mục còn mở cần có con người

Kiểm toán khả năng truy cập thủ công với người dùng công nghệ hỗ trợ (T48), việc xem xét bản dịch bởi người bản ngữ, và
việc có người chịu trách nhiệm cho các nhiệm vụ xem xét và cảnh báo ở trên.
Xem [tasks.md](../../../../spec/tasks.md).
