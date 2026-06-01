# Platform

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**abbreviation** | Option<**String**> | An abbreviation of the platform name | [optional]
**alternative_name** | Option<**String**> | An alternative name for the platform | [optional]
**category** | Option<[**models::PlatformCategoryEnums**](PlatformCategoryEnums.md)> |  | [optional]
**platform_type** | Option<**i32**> | The IGDB object unique identifier | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**generation** | Option<**i32**> | The generation of the platform | [optional]
**name** | Option<**String**> | The name of the platform | [optional]
**platform_family** | Option<**i32**> | The IGDB object unique identifier | [optional]
**platform_logo** | Option<**i32**> | The IGDB object unique identifier | [optional]
**slug** | Option<**String**> | A url-safe, unique, lower-case version of the name | [optional]
**summary** | Option<**String**> | The summary of the first Version of this platform | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**versions** | Option<**Vec<models::Id>**> |  | [optional]
**websites** | Option<**Vec<models::Id>**> |  | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


