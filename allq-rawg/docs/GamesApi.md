# \GamesApi

All URIs are relative to *https://api.rawg.io/api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**games_achievements_read**](GamesApi.md#games_achievements_read) | **GET** /games/{id}/achievements | Get a list of game achievements.
[**games_additions_list**](GamesApi.md#games_additions_list) | **GET** /games/{game_pk}/additions | Get a list of DLC's for the game, GOTY and other editions, companion apps, etc.
[**games_development_team_list**](GamesApi.md#games_development_team_list) | **GET** /games/{game_pk}/development-team | Get a list of individual creators that were part of the development team.
[**games_game_series_list**](GamesApi.md#games_game_series_list) | **GET** /games/{game_pk}/game-series | Get a list of games that are part of the same series.
[**games_list**](GamesApi.md#games_list) | **GET** /games | Get a list of games.
[**games_movies_read**](GamesApi.md#games_movies_read) | **GET** /games/{id}/movies | Get a list of game trailers.
[**games_parent_games_list**](GamesApi.md#games_parent_games_list) | **GET** /games/{game_pk}/parent-games | Get a list of parent games for DLC's and editions.
[**games_read**](GamesApi.md#games_read) | **GET** /games/{id} | Get details of the game.
[**games_reddit_read**](GamesApi.md#games_reddit_read) | **GET** /games/{id}/reddit | Get a list of most recent posts from the game's subreddit.
[**games_screenshots_list**](GamesApi.md#games_screenshots_list) | **GET** /games/{game_pk}/screenshots | Get screenshots for the game.
[**games_stores_list**](GamesApi.md#games_stores_list) | **GET** /games/{game_pk}/stores | Get links to the stores that sell the game.
[**games_suggested_read**](GamesApi.md#games_suggested_read) | **GET** /games/{id}/suggested | Get a list of visually similar games, available only for business and enterprise API users.
[**games_twitch_read**](GamesApi.md#games_twitch_read) | **GET** /games/{id}/twitch | Get streams on Twitch associated with the game, available only for business and enterprise API users.
[**games_youtube_read**](GamesApi.md#games_youtube_read) | **GET** /games/{id}/youtube | Get videos from YouTube associated with the game, available only for business and enterprise API users.



## games_achievements_read

> models::ParentAchievement games_achievements_read(id)
Get a list of game achievements.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::ParentAchievement**](ParentAchievement.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_additions_list

> models::GamesList200Response games_additions_list(game_pk, page, page_size)
Get a list of DLC's for the game, GOTY and other editions, companion apps, etc.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesList200Response**](games_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_development_team_list

> models::GamesDevelopmentTeamList200Response games_development_team_list(game_pk, ordering, page, page_size)
Get a list of individual creators that were part of the development team.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**ordering** | Option<**String**> | Which field to use when ordering the results. |  |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesDevelopmentTeamList200Response**](games_development_team_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_game_series_list

> models::GamesList200Response games_game_series_list(game_pk, page, page_size)
Get a list of games that are part of the same series.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesList200Response**](games_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_list

> models::GamesList200Response games_list(page, page_size, search, search_precise, search_exact, parent_platforms, platforms, stores, developers, publishers, genres, tags, creators, dates, updated, platforms_count, metacritic, exclude_collection, exclude_additions, exclude_parents, exclude_game_series, exclude_stores, ordering)
Get a list of games.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |
**search** | Option<**String**> | Search query. |  |
**search_precise** | Option<**bool**> | Disable fuzziness for the search query. |  |
**search_exact** | Option<**bool**> | Mark the search query as exact. |  |
**parent_platforms** | Option<**String**> | Filter by parent platforms, for example: `1,2,3`. |  |
**platforms** | Option<**String**> | Filter by platforms, for example: `4,5`. |  |
**stores** | Option<**String**> | Filter by stores, for example: `5,6`. |  |
**developers** | Option<**String**> | Filter by developers, for example: `1612,18893` or `valve-software,feral-interactive`. |  |
**publishers** | Option<**String**> | Filter by publishers, for example: `354,20987` or `electronic-arts,microsoft-studios`. |  |
**genres** | Option<**String**> | Filter by genres, for example: `4,51` or `action,indie`. |  |
**tags** | Option<**String**> | Filter by tags, for example: `31,7` or `singleplayer,multiplayer`. |  |
**creators** | Option<**String**> | Filter by creators, for example: `78,28` or `cris-velasco,mike-morasky`. |  |
**dates** | Option<**String**> | Filter by a release date, for example: `2010-01-01,2018-12-31.1960-01-01,1969-12-31`. |  |
**updated** | Option<**String**> | Filter by an update date, for example: `2020-12-01,2020-12-31`. |  |
**platforms_count** | Option<**i32**> | Filter by platforms count, for example: `1`. |  |
**metacritic** | Option<**String**> | Filter by a metacritic rating, for example: `80,100`. |  |
**exclude_collection** | Option<**i32**> | Exclude games from a particular collection, for example: `123`. |  |
**exclude_additions** | Option<**bool**> | Exclude additions. |  |
**exclude_parents** | Option<**bool**> | Exclude games which have additions. |  |
**exclude_game_series** | Option<**bool**> | Exclude games which included in a game series. |  |
**exclude_stores** | Option<**String**> | Exclude stores, for example: `5,6`. |  |
**ordering** | Option<**String**> | Available fields: `name`, `released`, `added`, `created`, `updated`, `rating`, `metacritic`. You can reverse the sort order adding a hyphen, for example: `-released`. |  |

### Return type

[**models::GamesList200Response**](games_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_movies_read

> models::Movie games_movies_read(id)
Get a list of game trailers.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::Movie**](Movie.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_parent_games_list

> models::GamesList200Response games_parent_games_list(game_pk, page, page_size)
Get a list of parent games for DLC's and editions.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesList200Response**](games_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_read

> models::GameSingle games_read(id)
Get details of the game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::GameSingle**](GameSingle.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_reddit_read

> models::Reddit games_reddit_read(id)
Get a list of most recent posts from the game's subreddit.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::Reddit**](Reddit.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_screenshots_list

> models::GamesScreenshotsList200Response games_screenshots_list(game_pk, ordering, page, page_size)
Get screenshots for the game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**ordering** | Option<**String**> | Which field to use when ordering the results. |  |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesScreenshotsList200Response**](games_screenshots_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_stores_list

> models::GamesStoresList200Response games_stores_list(game_pk, ordering, page, page_size)
Get links to the stores that sell the game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**game_pk** | **String** |  | [required] |
**ordering** | Option<**String**> | Which field to use when ordering the results. |  |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::GamesStoresList200Response**](games_stores_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_suggested_read

> models::GameSingle games_suggested_read(id)
Get a list of visually similar games, available only for business and enterprise API users.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::GameSingle**](GameSingle.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_twitch_read

> models::Twitch games_twitch_read(id)
Get streams on Twitch associated with the game, available only for business and enterprise API users.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::Twitch**](Twitch.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## games_youtube_read

> models::Youtube games_youtube_read(id)
Get videos from YouTube associated with the game, available only for business and enterprise API users.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** | An ID or a slug identifying this Game. | [required] |

### Return type

[**models::Youtube**](Youtube.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

