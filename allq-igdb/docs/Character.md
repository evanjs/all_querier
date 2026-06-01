# Character

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**akas** | Option<**Vec<String>**> | Alternative names for a character | [optional]
**country_name** | Option<**String**> | A Character's country of origin | [optional]
**created_at** | Option<**i64**> | Date this was initially added to the IGDB database | [optional]
**description** | Option<**String**> | Text describing a character | [optional]
**games** | Option<**Vec<models::Id>**> |  | [optional]
**gender** | Option<[**models::CharacterGenderEnum**](CharacterGenderEnum.md)> |  | [optional]
**character_gender** | Option<**i32**> | The IGDB object unique identifier | [optional]
**mug_shot** | Option<**i32**> | The IGDB object unique identifier | [optional]
**name** | Option<**String**> | The character's name | [optional]
**slug** | Option<**String**> | A url-safe, unique, lower-case version of the name | [optional]
**species** | Option<[**models::CharacterSpeciesEnum**](CharacterSpeciesEnum.md)> |  | [optional]
**character_species** | Option<**i32**> | The IGDB object unique identifier | [optional]
**updated_at** | Option<**i64**> | The last date this entry was updated in the IGDB database | [optional]
**url** | Option<**String**> | The website address (URL) of the item | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


