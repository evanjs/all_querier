# Region

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**name** | Option<**String**> | The name of the region | [optional]
**category** | Option<**Category**> | Whether the region is a local or continent (enum: locale, continent) | [optional]
**identifier** | Option<**serde_json::Value**> | This is the identifier of each region | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


