# \PlatformsApi

All URIs are relative to *https://api.rawg.io/api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**platforms_list**](PlatformsApi.md#platforms_list) | **GET** /platforms | Get a list of video game platforms.
[**platforms_lists_parents_list**](PlatformsApi.md#platforms_lists_parents_list) | **GET** /platforms/lists/parents | Get a list of parent platforms.
[**platforms_read**](PlatformsApi.md#platforms_read) | **GET** /platforms/{id} | Get details of the platform.



## platforms_list

> models::PlatformsList200Response platforms_list(ordering, page, page_size)
Get a list of video game platforms.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**ordering** | Option<**String**> | Which field to use when ordering the results. |  |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::PlatformsList200Response**](platforms_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## platforms_lists_parents_list

> models::PlatformsListsParentsList200Response platforms_lists_parents_list(ordering, page, page_size)
Get a list of parent platforms.

For instance, for PS2 and PS4 the “parent platform” is PlayStation.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**ordering** | Option<**String**> | Which field to use when ordering the results. |  |
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::PlatformsListsParentsList200Response**](platforms_lists_parents_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## platforms_read

> models::PlatformSingle platforms_read(id)
Get details of the platform.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** | A unique integer value identifying this Platform. | [required] |

### Return type

[**models::PlatformSingle**](PlatformSingle.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

