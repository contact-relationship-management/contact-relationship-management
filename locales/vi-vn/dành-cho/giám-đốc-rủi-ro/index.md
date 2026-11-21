# Dành cho Giám đốc rủi ro

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Vị thế rủi ro

**Tổng thể: chưa sẵn sàng cho dữ liệu chịu quản lý.** Phần mềm được kiểm soát tốt cho giai đoạn của nó, nhưng các rủi ro
còn lại chính chưa được giảm nhẹ bằng kiểm thử và quy trình mà chỉ tổ chức áp dụng hoặc các bên bên ngoài mới cung cấp được.

## Các rủi ro hàng đầu

| # | Rủi ro | Cơ sở xếp hạng | Giảm nhẹ trong kho mã | Còn lại |
|---|---|---|---|---|
| 1 | Lỗ hổng chưa được phát hiện | Chưa có kiểm thử xâm nhập độc lập (T42) | Kiểm thử của chính chúng tôi, các cổng chuỗi cung ứng, header/CSP nghiêm ngặt | **Cao** cho đến khi được kiểm thử |
| 2 | Lộ các trường nhạy cảm cho sai vai trò | Quy tắc truy cập phức tạp | Che theo vai trò trên bản ghi, tổng hợp và insight; một bài kiểm thử ma trận persona; một lỗ rò rỉ được tìm thấy và sửa ở chi tiết tài khoản (T47) | Trung bình: việc che đã được kiểm thử, chưa được xác minh độc lập |
| 3 | Nội bộ lạm dụng truy cập khẩn cấp | Break-glass nâng quyền đọc | Lý do bằng văn bản, giới hạn 60 phút, chỉ đọc, bắt buộc người thứ hai xem xét, được kiểm toán | Trung bình: chưa nối cảnh báo; không thể thu hồi |
| 4 | Vết kiểm toán bị sửa | Cần quyền truy cập cơ sở dữ liệu | Chuỗi hash cùng trigger chỉ ghi thêm; báo cáo chỗ đứt đầu tiên | Thấp-Trung bình: hash đầu chuỗi phải được neo bên ngoài cơ sở dữ liệu |
| 5 | Tiếp thị bất hợp pháp | Lỗi về đồng ý | Đường gửi thực thi sự đồng ý, không phải giao diện; lịch sử đồng ý bất biến | Trung bình cho đến khi một nhà cung cấp thật được tích hợp và rà soát (T39) |
| 6 | Tính sẵn sàng / lạm dụng | API công khai | Giới hạn tốc độ và khóa, chia sẻ qua Redis | Trung bình: giới hạn quay về theo từng bản sao nếu Redis hỏng |
| 7 | Không tuân thủ khả năng truy cập | Nghĩa vụ khu vực công | Quét axe tự động; tuyên bố khả năng truy cập trung thực | **Chưa định lượng** cho đến khi có kiểm toán thủ công (T48) |
| 8 | Chuỗi cung ứng | Các phụ thuộc và image | `cargo deny/audit`, `pnpm audit`, SBOM, digest được ghim và làm mới bởi Dependabot, một quy trình phát hành ký không cần khóa | Trung bình: việc ký chưa chạy trên bản phát hành thật (T49) |
| 9 | Bản ghi trùng lặp do thử lại | Khóa idempotency là tùy chọn ở API (T52) | Phát lại cho các máy khách gửi khóa; giao diện đi kèm luôn gửi | Thấp |
| 10 | Phân loại sai dữ liệu | Bên triển khai quyết định "liên hệ" là gì | Ghi chú phạm vi và mẫu DPIA | Tùy thuộc vào bạn |

## Các khoảng trống vốn dĩ vẫn mở

T39 (nhà cung cấp email thật), T42 (bảo đảm độc lập), T48 (kiểm toán khả năng truy cập thủ công),
T49 (bản phát hành được ký đầu tiên, việc người rà soát kiểm tra bản nháp API). Xem [tasks.md](../../../../spec/tasks.md)
và [plan.md](../../../../spec/plan.md).

## Hành động rủi ro đề xuất

Đăng ký chính thức các rủi ro 1, 7 và 10; yêu cầu T42 trước mọi dữ liệu thật; chỉ định
người chịu trách nhiệm xem xét và cảnh báo break-glass; đặt nhịp neo đầu chuỗi kiểm toán.
