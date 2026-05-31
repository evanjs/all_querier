# Game

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> |  | [optional][readonly]
**slug** | Option<**String**> |  | [optional][readonly]
**name** | Option<**String**> |  | [optional][readonly]
**released** | Option<**chrono::NaiveDate**> |  | [optional][readonly]
**tba** | Option<**bool**> |  | [optional][readonly]
**background_image** | Option<**String**> |  | [optional][readonly]
**rating** | **f64** |  | 
**rating_top** | Option<**i32**> |  | [optional][readonly]
**ratings** | Option<**serde_json::Value**> |  | [optional][readonly]
**ratings_count** | Option<**i32**> |  | [optional][readonly]
**reviews_text_count** | Option<**String**> |  | [optional][readonly]
**added** | Option<**i32**> |  | [optional][readonly]
**added_by_status** | Option<**serde_json::Value**> |  | [optional][readonly]
**metacritic** | Option<**i32**> |  | [optional][readonly]
**playtime** | Option<**i32**> | in hours | [optional][readonly]
**suggestions_count** | Option<**i32**> |  | [optional][readonly]
**updated** | Option<**chrono::DateTime<chrono::FixedOffset>**> |  | [optional][readonly]
**esrb_rating** | Option<[**models::GameEsrbRating**](GameEsrbRating.md)> |  | [optional]
**platforms** | Option<[**Vec<models::GamePlatformsInner>**](GamePlatformsInner.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


