# \CreatorRolesApi

All URIs are relative to *https://api.rawg.io/api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**creator_roles_list**](CreatorRolesApi.md#creator_roles_list) | **GET** /creator-roles | Get a list of creator positions (jobs).



## creator_roles_list

> models::CreatorRolesList200Response creator_roles_list(page, page_size)
Get a list of creator positions (jobs).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | A page number within the paginated result set. |  |
**page_size** | Option<**i32**> | Number of results to return per page. |  |

### Return type

[**models::CreatorRolesList200Response**](creator_roles_list_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

