# Dành cho Giám đốc chiến lược

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Câu hỏi chiến lược

Nó dùng để làm gì? Hôm nay nó là một **CRM tham chiếu đặt đặc tả lên trước** với một lớp bảo mật, quyền riêng tư và kiểm
toán đầy đủ bất thường. Nó không phải một sản phẩm có khách hàng, và nó không cạnh tranh với các CRM trưởng thành
([COMPARISONS.md](../../../../COMPARISONS.md)). Lựa chọn đặt trước bạn là
giữ nó làm một tham chiếu, hay đầu tư để nó có thể triển khai trong các môi trường chịu quản lý.

## Điều gì thực sự khác biệt

| Điểm mạnh | Vì sao có thể quan trọng | Mức vững chắc |
|---|---|---|
| Đặc tả trước: một đặc tả điều khiển cả dịch vụ Rust và giao diện Svelte ([spec](../../../../spec/index.md)) | Rẻ hơn để kiểm toán, chuyển đổi và giữ nhất quán | Thật; các bài kiểm thử trích dẫn đặc tả |
| Tiếp thị đặt sự đồng ý lên trước, được thực thi trong đường gửi | Yêu cầu cứng với khách hàng y tế và khu vực công | Thật, nhưng chưa có bộ gửi thật (T39) |
| Chấm điểm và dự báo khách hàng tiềm năng dựa trên quy tắc, giải thích được | Khách hàng phải biện minh các quyết định về con người | Thật; cố ý không dùng học máy |
| Liên hệ được lấy từ một dịch vụ định danh, không nhân bản | Giữ ít dữ liệu cá nhân hơn, bề mặt rò rỉ nhỏ hơn | Thật; phụ thuộc vào việc dịch vụ đó tồn tại |
| Kiểm toán cho thấy dấu hiệu giả mạo, che theo vai trò, truy cập khẩn cấp có lý do | Các biện pháp kiểm soát mà khách hàng chịu quản lý hỏi đầu tiên | Đã triển khai và kiểm thử; chưa được xác minh độc lập |

## Điều gì không khác biệt

Độ rộng tính năng. Báo giá, hợp đồng, điện thoại, chấm điểm bằng học máy và
thanh toán nằm ngoài phạm vi ([scope](../../../../spec/scope.md)). Ai chọn
một CRM theo tính năng sẽ chọn cái khác.

## Các lựa chọn

| Lựa chọn | Ý nghĩa | Chi phí và rủi ro | Bằng chứng cần có |
|---|---|---|---|
| A. Giữ làm một tham chiếu | Giữ nó làm thư viện học tập và mẫu | Thấp nhất; giá trị là danh tiếng | Không cần gì thêm ngoài hôm nay |
| B. Ngách khu vực chịu quản lý | Hoàn tất các hạng mục còn mở và bán hoặc hỗ trợ cho các cơ quan y tế và công quyền | Cao nhất; cần một nhà cung cấp, điều khoản pháp lý, kiểm thử bên ngoài, bảo đảm liên tục | Kiểm thử độc lập (T42), kiểm toán khả năng truy cập (T48), email thật (T39) |
| C. Thành phần, không phải sản phẩm | Cung cấp chuỗi kiểm toán, việc che và các mẫu break-glass dưới dạng các mảnh tái sử dụng | Vừa; cần đóng gói và tài liệu | Công việc tách ra; không có chứng nhận mới |

**Khuyến nghị: làm A ngay bây giờ, và tài trợ cho phép thử rẻ nhất của B trước khi cam kết.**
Đánh giá của NHS kết luận rằng sản phẩm có khả năng nằm ngoài phạm vi DTAC
([đánh giá](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
nên con đường ra thị trường chưa ngã ngũ. Hãy chi tiền trước hết để tìm ra khách hàng nào
cần cái này và họ sẽ đòi hỏi gì, không phải cho các chứng nhận mà chưa ai yêu cầu.

## Lộ trình, theo thứ tự mà bằng chứng gợi ý

1. Kiểm thử bảo mật độc lập và kiểm toán khả năng truy cập thủ công (T42, T48): chúng
   quyết định liệu B có khả thi hay không.
2. Nhà cung cấp email thật (T39) khi đã chọn nhà cung cấp và điều khoản xử lý dữ liệu;
   điểm nối của bộ gửi và một đường SMTP đã có và chạy với một máy chủ thử cục bộ, cùng liên kết hủy đăng ký đã ký, danh sách chặn và một webhook kết quả đã ký chung, nhưng chưa tích hợp nhà cung cấp nào ([roadmap](../../../../spec/roadmap.md)).
3. Chứng minh việc ký bản phát hành bằng một bản phát hành được gắn thẻ đầu tiên (T49): điều kiện tối thiểu cho
   mua sắm. Quy trình đã xây dựng; nó chưa chạy.
4. Chỉ sau đó mới đến các tính năng sản phẩm trong lộ trình (ghi nhận nguồn, vùng phụ trách,
   đồng hồ SLA theo giờ làm việc, cổng cơ sở tri thức công khai).

## Rủi ro chiến lược

- **Tuyên bố nhiều hơn sự thật.** Cách nhanh nhất để mất một khách hàng chịu quản lý là một
  tuyên bố tuân thủ mà bằng chứng không ủng hộ. Xem [bản tóm tắt GRC](../quản-trị-rủi-ro-tuân-thủ/index.md).
- **Bảo trì.** Công việc bảo mật không bao giờ xong: cập nhật phụ thuộc, làm mới digest,
  xoay khóa, kiểm thử lại. Lựa chọn B ràng buộc bạn với điều đó.
- **Phụ thuộc vào một dịch vụ định danh** nằm ngoài kho mã này.
- **Một người bảo trì.** Dự án có một người bảo trì duy nhất và nói rằng độ sâu của việc
  xem xét là của một người ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Lựa chọn B cần nhiều hơn một người
  có thể chịu trách nhiệm về công việc bảo mật.

## Các quyết định dành cho bạn

1. Chọn A, B hay C, và ngày xem xét lại lựa chọn đó.
2. Khách hàng mục tiêu đầu tiên là ai, tìm ra bằng trò chuyện trước khi chi tiền.
3. Có tài trợ cho bảo đảm độc lập ngay bây giờ hay không (xem [bản tóm tắt Giám đốc điều hành](../giám-đốc-điều-hành/index.md)).
