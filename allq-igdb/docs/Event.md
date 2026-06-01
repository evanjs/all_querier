# Event

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**name** | Option<**String**> | The name of the event | [optional]
**description** | Option<**String**> | The description of the event | [optional]
**slug** | Option<**String**> | A url-safe, unique, lower-case version of the name | [optional]
**event_logo** | Option<**i32**> | The IGDB object unique identifier | [optional]
**start_time** | Option<**i64**> | Start time of the event in UTC | [optional]
**end_time** | Option<**i64**> | End time of the event in UTC | [optional]
**time_zone** | Option<**String**> | Timezone the event is in. | [optional]
**live_stream_url** | Option<**String**> | URL to the livestream of the event. | [optional]
**games** | Option<**Vec<models::Id>**> |  | [optional]
**videos** | Option<**Vec<models::Id>**> |  | [optional]
**event_networks** | Option<**Vec<models::Id>**> |  | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


