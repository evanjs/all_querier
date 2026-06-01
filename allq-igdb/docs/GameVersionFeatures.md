# GameVersionFeatures

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**category** | Option<[**models::GameVersionFeatureCategoryEnums**](GameVersionFeatureCategoryEnums.md)> |  | [optional]
**description** | Option<**String**> | The description of the feature | [optional]
**position** | Option<**i32**> | Position of this feature in the list of features | [optional]
**title** | Option<**String**> | The title of the version/addition/DLC | [optional]
**values** | Option<**Vec<models::Id>**> | The bool/text value of the feature | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


