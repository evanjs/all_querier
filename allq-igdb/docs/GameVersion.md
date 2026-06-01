# GameVersion

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**features** | Option<**Vec<models::Id>**> | Features and descriptions of what makes each version/edition different from the main game | [optional]
**game** | Option<**i32**> | The IGDB object unique identifier | [optional]
**games** | Option<**Vec<models::Id>**> | Game Versions and Editions | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


