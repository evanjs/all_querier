# GameTimeToBeat

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**game_id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**hastily** | Option<**i32**> | Average time (in seconds) to finish the game to its credits without spending notable time on extras such as side quests. | [optional]
**normally** | Option<**i32**> | Average time (in seconds) to finish the game while mixing in some extras such as side quests without being overly thorough. | [optional]
**completely** | Option<**i32**> | Average time (in seconds) to finish the game to 100% completion. | [optional]
**count** | Option<**i32**> | Total number of time to beat submissions for this game | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


