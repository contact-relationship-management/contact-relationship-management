# Dành cho Giám đốc điều hành

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Quyết định trong một đoạn

Đây là một CRM ở mức demo với nền tảng bảo mật và quyền riêng tư mạnh bất thường. Nó **chưa sẵn sàng** để lưu dữ liệu
thật của bệnh nhân hay công dân, và chính kho mã đã nói như vậy ([plan.md](../../../../spec/plan.md)). Áp dụng nó là
quyết định tài trợ cho phần việc bên ngoài còn lại, không phải quyết định bật nó lên.

## Những gì đã có

- Một CRM hoạt động được trong bán hàng, tiếp thị, hỗ trợ và phân tích, với API Rust và giao diện web, được xây dựng
  theo một đặc tả bằng văn bản.
- Tiếp thị đặt sự đồng ý lên hàng đầu, kiểm soát truy cập che các giá trị nhạy cảm với những vai trò không nên thấy,
  một vết kiểm toán cho thấy dấu hiệu giả mạo, truy cập dữ liệu cá nhân và xóa dữ liệu cho các yêu cầu về quyền riêng
  tư, và truy cập khẩn cấp ("break-glass") có lý do, có thời hạn và được xem xét lại.
- 54 trên 58 hạng mục công việc được theo dõi đã hoàn thành ([tasks.md](../../../../spec/tasks.md)).

## Những gì còn ngăn cản việc dùng thật

| Rào cản | Vì sao quan trọng | Ai có thể giải quyết |
|---|---|---|
| Chưa có gửi email thật (T39) | Tiếp thị chỉ được mô phỏng; một nhà cung cấp thật cần hợp đồng và thỏa thuận xử lý dữ liệu | Bạn chọn nhà cung cấp; bộ phận pháp chế ký |
| Chưa có kiểm thử bảo mật độc lập (T42) | Bệnh viện và khách hàng chính phủ thường yêu cầu; các bài kiểm thử của chính chúng tôi không độc lập | Một công ty bên ngoài |
| Chưa có kiểm toán khả năng truy cập thủ công (T48) | Các cơ quan công quyền phải công bố một tuyên bố về khả năng truy cập mà họ có thể bảo đảm | Những người dùng công nghệ hỗ trợ |
| Việc ký bản phát hành đã được xây dựng nhưng chưa chạy trên bản phát hành thật, và chưa có người rà soát nào kiểm tra bản nháp tiêu chuẩn API (T49) | Danh sách kiểm tra mua sắm yêu cầu cả hai | Kỹ thuật (gắn thẻ bản phát hành đầu tiên); một người rà soát bên ngoài |

## Những quyết định thuộc về bạn

1. **Thị trường nào trước.** Đánh giá theo Tiêu chí đánh giá công nghệ số của NHS
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   kết luận rằng sản phẩm có khả năng nằm ngoài phạm vi DTAC và sẽ không đạt các phần áp dụng do thiếu bằng chứng ở cấp
   tổ chức (cán bộ an toàn lâm sàng được chỉ định, các chứng nhận). Hãy chọn con đường trước khi chi tiền cho nó.
2. **Có tài trợ cho bảo đảm độc lập hay không** (T42, T48). Nếu không, tuyên bố trung thực là
   "xây dựng theo tiêu chuẩn cao, chưa được người ngoài kiểm thử".
3. **Ai chịu trách nhiệm về rủi ro** một khi giữ dữ liệu thật. Xem các bản tóm tắt cho [Giám đốc rủi ro](../giám-đốc-rủi-ro/index.md)
   và [Giám đốc pháp chế](../giám-đốc-pháp-chế/index.md).

## Không nên nói gì ra bên ngoài

Đừng tuyên bố tuân thủ, chứng nhận, "tuân thủ HIPAA" hay "được NHS phê duyệt".
Bộ bằng chứng ([compliance](../../../../spec/compliance/index.md)) hỗ trợ một tuyên bố hẹp hơn và đúng sự thật: các biện
pháp kiểm soát đã được ánh xạ, những biện pháp đã triển khai được kiểm thử, và các khoảng trống được nêu tên.
