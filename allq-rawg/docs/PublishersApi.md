# \PublishersApi

All URIs are relative to *https://api.rawg.io/api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**publishers_list**](PublishersApi.md#publishers_list) | **GET** /publishers | Get a list of video game publishers.
[**publishers_read**](PublishersApi.md#publishers_read) | **GET** /publishers/{id} | Get details of the publisher.



## publishers_list

> models::PublishersList200Response publishers_list(page, page_size)
Get a list of video game publishers.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::PublishersList200Response**](publishers_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## publishers_read

> models::PublisherSingle publishers_read(id)
Get details of the publisher.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **i32** | A unique integer value identifying this Publisher. | [required] |

### Return type

[**models::PublisherSingle**](PublisherSingle.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

