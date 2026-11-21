# Người kiểm thử bảo mật độc lập

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Vì sao trang này tồn tại

Mọi biện pháp kiểm soát bảo mật trong dự án này đều do các tác giả xây dựng và kiểm thử
(CRM-T42). Điều đó cho thấy các biện pháp hoạt động đúng như các tác giả hiểu. Nó không
cho thấy không thiếu gì. Bệnh viện và khách hàng chính phủ thường yêu cầu
một cuộc kiểm thử bởi người độc lập trước khi họ giữ dữ liệu thật, và tuyên bố trung thực
cho đến lúc đó là "xây dựng theo tiêu chuẩn cao, chưa được người ngoài kiểm thử"
([bản tóm tắt Giám đốc điều hành](../giám-đốc-điều-hành/index.md),
[bản tóm tắt an ninh](../giám-đốc-an-ninh-thông-tin/index.md)).

Trang này là thứ cần đưa cho một công ty kiểm thử, và những gì cần mong đợi nhận lại.

## Đạt nghĩa là gì, và không nghĩa là gì

Một báo cáo sạch nói rằng một đội được nêu tên, trong một thời gian được nêu rõ, với một phạm vi được nêu rõ, không
tìm thấy vấn đề nghiêm trọng. Nó **không** phải là chứng nhận, không phải là công nhận và không phải là
bảo đảm. Đừng mô tả hệ thống là "an toàn", "được chứng nhận" hay "đã kiểm thử xâm nhập
theo một tiêu chuẩn" vượt quá những gì báo cáo nêu
([bản tóm tắt GRC](../quản-trị-rủi-ro-tuân-thủ/index.md)).

## Chọn người kiểm thử

- Độc lập với các tác giả, và với bất kỳ ai đã cấu hình bản triển khai.
- Có các chuyên gia kiểm thử được nêu tên, giàu kinh nghiệm, với chứng nhận liên quan cho
  thị trường của bạn nếu có (ví dụ CREST, hoặc chương trình NCSC CHECK cho
  công việc của chính phủ Anh). Hãy hỏi chứng nhận nào bao phủ các cá nhân thực hiện
  công việc, không chỉ công ty.
- Thỏa thuận bằng văn bản về phạm vi, quy tắc tham gia, bảo mật, xử lý dữ liệu
  và trách nhiệm pháp lý trước khi bắt đầu bất kỳ kiểm thử nào. Hợp đồng do bộ phận
  pháp chế thỏa thuận ([bản tóm tắt Giám đốc pháp chế](../giám-đốc-pháp-chế/index.md)).

## Cần kiểm thử gì

| Lĩnh vực | Những gì người kiểm thử nên thử | Nơi mô tả thiết kế |
|---|---|---|
| Xác thực | Token giả mạo, hết hạn, phát lại và sai đối tượng; hạ cấp xuống không xác thực; kiểm tra khi khởi động từ chối chạy nếu xác thực bị tắt | [auth](../../../../spec/auth.md) |
| Phân quyền | Chạm tới các bản ghi hoặc hành động mà một vai trò không nên: leo thang ngang và dọc, kiểm tra ở cấp bản ghi, hành động hủy hoại, các lần đọc chỉ dành cho quản trị viên (kiểm toán, lưu giữ, xem xét break-glass) | [auth](../../../../spec/auth.md) |
| Lộ dữ liệu | Giá trị bị che rò rỉ qua danh sách, các lần đọc chi tiết gộp, insight, tổng hợp, thông báo lỗi, bộ nhớ đệm hoặc khóa bộ nhớ đệm | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Truy cập khẩn cấp | Dùng, gia hạn hoặc lạm dụng break-glass; tự xem xét quyền của chính mình; xem xét hai lần | [auth](../../../../spec/auth.md) (T44) |
| Kiểm toán | Sửa, xóa hoặc sắp xếp lại lịch sử; vượt qua chuỗi; giấu các lần đọc | [audit](../../../../spec/audit.md) |
| Khả năng chống lạm dụng | Vượt qua giới hạn tốc độ và khóa, khóa những người dùng khác, giả mạo header cho khóa máy khách | T43 |
| Xử lý đầu vào | Tiêm mã (SQL, JSON, header), nội dung quá khổ, con trỏ và khóa idempotency sai dạng, phát lại hoặc va chạm khóa giữa các người gọi | T51, T52 |
| Front end web | Cross-site scripting, giả mạo yêu cầu, xử lý phiên, proxy backend-for-frontend, chính sách bảo mật nội dung, clickjacking | UI `hooks.server.ts` |
| Hạ tầng như được giao | Image container, cổng mở, khả năng truy cập Redis và PostgreSQL, xử lý bí mật, giá trị cấu hình mặc định | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Chuỗi cung ứng | Nguồn gốc phụ thuộc và image, các SBOM, bản build không ký | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Logic nghiệp vụ | Vượt qua sự đồng ý trong đường gửi, việc xóa để sót dữ liệu, thao túng điểm số, tạo trùng lặp | [regulatory](../../../../spec/regulatory.md) |

**Ngoài phạm vi trừ khi bạn thỏa thuận khác:** dịch vụ định danh phát hành
token (nó nằm ngoài kho mã này), nền tảng lưu trữ, và tấn công từ chối dịch vụ
ở quy mô lớn.

## Cần đưa gì cho người kiểm thử

1. Một **bản staging** khớp với những gì sẽ chạy trong sản xuất: cấu hình
   sản xuất, xác thực bật, TLS, Redis và PostgreSQL thật. Đừng
   kiểm thử các giá trị mặc định của bản demo.
2. **Chỉ dữ liệu tổng hợp.** Tác vụ nạp dữ liệu tạo một tập khách hàng không có người
   thật. Dữ liệu bệnh nhân hay công dân thật không bao giờ được có trong môi trường kiểm thử.
3. **Một tài khoản cho mỗi vai trò**, để có thể kiểm thử phân quyền từ mỗi phía. Chính sách
   tham chiếu là `config/abac-policy.reference.json`.
4. Các tài liệu được liên kết ở trên, tài liệu OpenAPI tại `/api-docs/openapi.json`,
   và quyền đọc kho mã, để cuộc kiểm thử có thể kết hợp hộp đen và rà soát
   mã.
5. Các **điểm yếu đã biết**, để người kiểm thử dành thời gian cho những gì chưa
   được viết ra:
   - việc ký bản phát hành đã xây dựng nhưng chưa chạy trên bản phát hành thật (T49);
   - mã hóa trường bao phủ hai cột với khóa cục bộ, không có KMS;
   - xác thực lại từng bước chưa được xây dựng;
   - break-glass không thể thu hồi, chỉ có thể hết hạn, và chưa nối cảnh báo nào;
   - việc tải lại chính sách xuất hiện trong insight được lưu đệm tối đa 30 giây;
   - đăng xuất khi không hoạt động được thực hiện ở phía máy khách;
   - trạng thái giới hạn tốc độ quay về theo từng bản sao nếu Redis hỏng.
   Hãy yêu cầu họ xác nhận hoặc bác bỏ từng điểm, không bỏ qua chúng.

## Quy tắc tham gia cần thỏa thuận bằng văn bản

- Người kiểm thử, ngày và địa chỉ nguồn được nêu tên; một đầu mối liên hệ 24 giờ phía bạn.
- Những gì được phép thử (ví dụ, không phá hủy kho dữ liệu kiểm thử
  khi chưa báo) và những gì phải dừng khi có bằng chứng về việc lộ dữ liệu.
- Ai được báo ngay về một phát hiện nghiêm trọng, và báo bằng cách nào, trước báo cáo cuối cùng.
- Cách lưu trữ và hủy báo cáo, bằng chứng và mọi ghi chú của người kiểm thử.

## Bạn nên nhận lại gì

- Một báo cáo có phạm vi, phương pháp, ngày, tên người kiểm thử và chứng nhận.
- Mỗi phát hiện kèm mức độ nghiêm trọng, bằng chứng, các bước tái hiện, thành phần bị ảnh hưởng
  và đề xuất cách sửa.
- Một tuyên bố rõ ràng về những gì **chưa** được kiểm thử.
- Một lần kiểm thử lại các phát hiện đã sửa, được bao gồm trong hợp đồng hoặc được định giá.

## Sau báo cáo

1. Phân loại từng phát hiện trong `spec/tasks.md` thành một tác vụ có người phụ trách và ngày.
2. Sửa các phát hiện nghiêm trọng và mức cao trước khi có bất kỳ dữ liệu thật nào, rồi nhờ người kiểm thử
   xác nhận các bản sửa.
3. Thêm một bài kiểm thử hồi quy cho mỗi bản sửa. Quy tắc của dự án là không làm yếu một bài kiểm thử
   hay cổng để cho một thứ vượt qua.
4. Ghi lại kết quả một cách trung thực: ngày báo cáo, phạm vi, các phát hiện và
   trạng thái của chúng. Cập nhật kế hoạch sẵn sàng ([plan](../../../../spec/plan.md)) và
   bộ tuân thủ ([compliance](../../../../spec/compliance/index.md)).
5. Kiểm thử lại sau thay đổi đáng kể, và theo lịch do người chịu trách nhiệm rủi ro của bạn đặt.
   Một lần kiểm thử là một ảnh chụp, không phải một trạng thái.

## Thứ tự công việc

1. Trước hết đóng những gì rẻ và đã biết, để người kiểm thử không tiêu ngân sách
   vào việc khám phá lại chúng: một bản phát hành đã ký đầu tiên (T49), cảnh báo cho break-glass
   và cho các lần xác minh kiểm toán thất bại, Redis sau TLS và thông tin xác thực.
2. Chuẩn bị bản staging, các tài khoản và dữ liệu nạp.
3. Thỏa thuận phạm vi, hợp đồng và quy tắc tham gia.
4. Kiểm thử; nhận ngay các phát hiện nghiêm trọng.
5. Sửa, kiểm thử lại và ghi lại.
6. Chỉ sau đó mới cập nhật các tuyên bố bên ngoài, và chỉ đến mức báo cáo ủng hộ.

## Các quyết định dành cho bạn

1. Ai đặt hàng và ai sở hữu các phát hiện (xem
   [bản tóm tắt Giám đốc rủi ro](../giám-đốc-rủi-ro/index.md)).
2. Chứng nhận nào mà thị trường của bạn mong đợi ở người kiểm thử.
3. Ngân sách và ngày, được đặt theo cái giá của sự chậm trễ thay vì theo điều
   thuận tiện.
4. Kiểm thử một lần trước khi ra mắt hay theo lịch định kỳ.
