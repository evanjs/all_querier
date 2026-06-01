# GameSingle

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> |  | [optional][readonly]
**slug** | Option<**String**> |  | [optional][readonly]
**name** | Option<**String**> |  | [optional][readonly]
**name_original** | Option<**String**> |  | [optional][readonly]
**description** | Option<**String**> |  | [optional][readonly]
**metacritic** | Option<**i32**> |  | [optional][readonly]
**metacritic_platforms** | Option<[**Vec<models::GamePlatformMetacritic>**](GamePlatformMetacritic.md)> |  | [optional][readonly]
**released** | Option<**chrono::NaiveDate**> |  | [optional][readonly]
**tba** | Option<**bool**> |  | [optional][readonly]
**updated** | Option<**chrono::DateTime<chrono::FixedOffset>**> |  | [optional][readonly]
**background_image** | Option<**String**> |  | [optional][readonly]
**background_image_additional** | Option<**String**> |  | [optional][readonly]
**website** | Option<**String**> |  | [optional][readonly]
**rating** | **f64** |  | 
**rating_top** | Option<**i32**> |  | [optional][readonly]
**ratings** | Option<**serde_json::Value**> |  | [optional][readonly]
**reactions** | Option<**serde_json::Value**> |  | [optional][readonly]
**added** | Option<**i32**> |  | [optional][readonly]
**added_by_status** | Option<**serde_json::Value**> |  | [optional][readonly]
**playtime** | Option<**i32**> | in hours | [optional][readonly]
**screenshots_count** | Option<**i32**> |  | [optional][readonly]
**movies_count** | Option<**i32**> |  | [optional][readonly]
**creators_count** | Option<**i32**> |  | [optional][readonly]
**achievements_count** | Option<**i32**> |  | [optional][readonly]
**parent_achievements_count** | Option<**String**> |  | [optional][readonly]
**reddit_url** | Option<**String**> | For example \"https://www.reddit.com/r/uncharted/\" or \"uncharted\" | [optional][readonly]
**reddit_name** | Option<**String**> |  | [optional][readonly]
**reddit_description** | Option<**String**> |  | [optional][readonly]
**reddit_logo** | Option<**String**> |  | [optional][readonly]
**reddit_count** | Option<**i32**> |  | [optional][readonly]
**twitch_count** | Option<**String**> |  | [optional][readonly]
**youtube_count** | Option<**String**> |  | [optional][readonly]
**reviews_text_count** | Option<**String**> |  | [optional][readonly]
**ratings_count** | Option<**i32**> |  | [optional][readonly]
**suggestions_count** | Option<**i32**> |  | [optional][readonly]
**alternative_names** | Option<**Vec<String>**> |  | [optional][readonly]
**metacritic_url** | Option<**String**> | For example \"http://www.metacritic.com/game/playstation-4/the-witcher-3-wild-hunt\" | [optional][readonly]
**parents_count** | Option<**i32**> |  | [optional][readonly]
**additions_count** | Option<**i32**> |  | [optional][readonly]
**game_series_count** | Option<**i32**> |  | [optional][readonly]
**esrb_rating** | Option<[**models::GameEsrbRating**](GameEsrbRating.md)> |  | [optional]
**platforms** | Option<[**Vec<models::GamePlatformsInner>**](GamePlatformsInner.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


