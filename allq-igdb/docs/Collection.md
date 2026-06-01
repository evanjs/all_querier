# Collection

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**as_child_relations** | Option<**Vec<models::Id>**> |  | [optional]
**as_parent_relations** | Option<**Vec<models::Id>**> |  | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**games** | Option<**Vec<models::Id>**> |  | [optional]
**name** | Option<**String**> | Umbrella term for a collection of games | [optional]
**slug** | Option<**String**> | A url-safe, unique, lower-case version of the name | [optional]
**r#type** | Option<**i32**> | The IGDB object unique identifier | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


