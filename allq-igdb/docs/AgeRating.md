# AgeRating

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | The IGDB object unique identifier | [optional]
**category** | Option<[**models::AgeRatingCategoryEnums**](AgeRatingCategoryEnums.md)> |  | [optional]
**content_descriptions** | Option<**Vec<models::Id>**> | Array of Age Rating Content Description IDs | [optional]
**rating** | Option<[**models::AgeRatingEnums**](AgeRatingEnums.md)> |  | [optional]
**rating_category** | Option<[**models::AgeRatingCategory**](AgeRatingCategory.md)> |  | [optional]
**organization** | Option<[**models::AgeRatingOrganization**](AgeRatingOrganization.md)> |  | [optional]
**rating_cover_url** | Option<**String**> | The url for the image of an age rating | [optional]
**synopsis** | Option<**String**> | A free text motivating a rating | [optional]
**checksum** | Option<**uuid::Uuid**> | Hash of the object | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


