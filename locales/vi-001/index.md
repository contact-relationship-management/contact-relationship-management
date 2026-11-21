# Contact Relationship Management

Phần mềm CRM mã nguồn mở cho bán hàng, tiếp thị và hỗ trợ khách hàng.

Contact Relationship Management theo dõi mối quan hệ với khách hàng và khách hàng tiềm năng trong bán hàng, tiếp thị và hỗ trợ. Phần mềm lưu một hồ sơ cho mỗi liên hệ và yêu cầu có sự đồng ý được ghi nhận trước khi gửi bất kỳ thông điệp nào. Phần mềm chưa sẵn sàng cho môi trường sản xuất: nó chạy trên dữ liệu tổng hợp.

## Lợi ích

- **Một hồ sơ liên hệ duy nhất.** Bán hàng, tiếp thị, hỗ trợ và phân tích dùng chung một hồ sơ.
- **Sự đồng ý được ghi nhận.** Không có gì được gửi đến một liên hệ nếu chưa có sự đồng ý được ghi nhận. Mỗi email có một liên kết hủy đăng ký đã được ký.
- **Giảm dữ liệu cá nhân.** CRM lưu một tham chiếu đến một người trong dịch vụ định danh, không lưu bản sao của tên hay địa chỉ.
- **Kết quả giải thích được.** Điểm khách hàng tiềm năng được tính từ các quy tắc dễ đọc. Dự báo được tính từ các giai đoạn quy trình bán hàng có trọng số.
- **Thiết kế quốc tế.** Tên, địa chỉ, lịch và hướng viết không bị gắn với định dạng của một quốc gia. Giao diện có sẵn bằng tám ngôn ngữ.
- **Đặc tả công khai.** Đặc tả, mã nguồn và các bài kiểm thử đều công khai. Tài liệu nêu rõ điều gì đã được xây dựng và điều gì chưa.

## Tính năng

- **Bán hàng.** Chấm điểm khách hàng tiềm năng theo quy tắc, quy trình giao dịch dạng bảng kanban và dự báo theo trọng số giai đoạn.
- **Tiếp thị.** Chiến dịch theo phân khúc và chuỗi nuôi dưỡng khách hàng. Việc gửi chỉ được mô phỏng trừ khi cấu hình một chế độ thư.
- **Hỗ trợ.** Phiếu hỗ trợ có mức ưu tiên và thời hạn mức dịch vụ, cùng các bài viết trong cơ sở tri thức.
- **Phân tích.** Bảng điều khiển về tỷ lệ thắng, quy trình theo giai đoạn, lợi tức chiến dịch, tình trạng mức dịch vụ và giá trị vòng đời khách hàng.
- **Chất lượng dữ liệu.** Phát hiện trùng lặp theo xác suất, có thể gộp và hoàn tác.
- **Quản trị.** Nhật ký kiểm toán, truy cập theo vai trò, và hỗ trợ các yêu cầu truy cập dữ liệu cá nhân và yêu cầu xóa.

## Bắt đầu

1. **Lấy mã nguồn.** Sao chép (clone) kho mã. Cần có Node.js và pnpm.
2. **Khởi động ứng dụng.** Trong thư mục Svelte, chạy `pnpm install`, rồi `pnpm dev`.
3. **Đăng nhập.** Dùng `example@example.com`. Không cần dịch vụ nào khác.
4. **Xem phần giới thiệu.** Phần giới thiệu bao gồm liên hệ, khách hàng tiềm năng, giao dịch, chiến dịch, phiếu hỗ trợ và góc nhìn điều hành, với dữ liệu hư cấu.

[Hướng dẫn](../../docs/tutorial-first-tour.md) mô tả từng bước. [Cài đặt và sử dụng](../../INSTALL.md) trình bày toàn bộ dịch vụ.

## So sánh

| Dự án | Loại | Khác biệt |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM mã nguồn mở | Sản phẩm đầy đủ. Dự án này là một bản trình diễn nhỏ hơn, dựa trên đặc tả. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM mã nguồn mở | Nhiều tính năng hơn. Dự án này nhấn mạnh các quy tắc giải thích được. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM mã nguồn mở | Trưởng thành và lớn. Dự án này là một tham chiếu gọn nhẹ. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM SaaS thương mại | Dự án này mô phỏng cùng một thực hành. Không chia sẻ mã nguồn hay dữ liệu. |

Dự án này là một bản trình diễn và không cạnh tranh với các sản phẩm trên. Xem [tất cả so sánh](../../COMPARISONS.md).

## Tóm tắt theo vai trò

[Tóm tắt theo vai trò](dành-cho/) cung cấp một trang cho mỗi vai trò cấp cao: phần mềm làm được gì, không làm được gì, và những quyết định nào thuộc về vai trò đó.

## Trạng thái

Phần mềm này chưa sẵn sàng cho môi trường sản xuất ([lộ trình lên sản xuất](lộ-trình-lên-sản-xuất/index.md)). Chỉ dùng dữ liệu tổng hợp. Không có email thật nào được gửi trừ khi cấu hình chế độ thư thật. Không có nội dung nào trên trang này là tư vấn pháp lý.
