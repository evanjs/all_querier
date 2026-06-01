# ExternalGame

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**category** | Option<[**models::ExternalGameCategoryEnums**](ExternalGameCategoryEnums.md)> |  | [optional]
**external_game_source** | Option<**i32**> | The IGDB object unique identifier | [optional]
**countries** | Option<**Vec<i32>**> | The ISO country code of the external game product. | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**game** | Option<**i32**> | The IGDB object unique identifier | [optional]
**media** | Option<[**models::ExternalGameMediaEnums**](ExternalGameMediaEnums.md)> |  | [optional]
**game_release_format** | Option<**i32**> | The IGDB object unique identifier | [optional]
**name** | Option<**String**> | The name of the game according to the other service | [optional]
**platform** | Option<**i32**> | The IGDB object unique identifier | [optional]
**uid** | Option<**String**> | The other services ID for this game | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**year** | Option<**i32**> | The year in full (2018) | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


