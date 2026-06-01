# Company

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**change_date_category** | Option<[**models::ChangeDateCategoryEnum**](ChangeDateCategoryEnum.md)> |  | [optional]
**change_date_format** | Option<**i32**> | The IGDB object unique identifier | [optional]
**change_date** | Option<**i64**> | The date when a compnay got a new ID | [optional]
**changed_company_id** | Option<**String**> | The new ID for a company that has gone through a merger or restructuring | [optional]
**country** | Option<**i32**> | ISO 3166-1 numeric country code | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**description** | Option<**String**> | A free text description of the company | [optional]
**developed** | Option<**Vec<models::Id>**> |  | [optional]
**logo** | Option<**i32**> | The IGDB object unique identifier | [optional]
**name** | Option<**String**> | The name of the company | [optional]
**parent** | Option<**i32**> | The IGDB object unique identifier | [optional]
**published** | Option<**Vec<models::Id>**> |  | [optional]
**slug** | Option<**String**> | A url-safe, unique, lower-case version of the name | [optional]
**start_date** | Option<**i64**> | The date that the company was founded | [optional]
**start_date_category** | Option<[**models::StartDateCategoryEnum**](StartDateCategoryEnum.md)> |  | [optional]
**start_date_format** | Option<**i32**> | The IGDB object unique identifier | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**websites** | Option<**Vec<models::Id>**> |  | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


